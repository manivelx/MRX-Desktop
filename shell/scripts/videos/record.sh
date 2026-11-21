#!/usr/bin/env bash
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/mrx-session.sh"
CONFIG_FILE="$HOME/.config/mrx/config.json"
[[ -f "$CONFIG_FILE" ]] || CONFIG_FILE="$HOME/.config/mrx/config.json"
JSON_PATH=".screenRecord.savePath"
CUSTOM_PATH=$(jq -r "$JSON_PATH" "$CONFIG_FILE" 2>/dev/null)
RECORDING_DIR=""
if [[ -n "$CUSTOM_PATH" ]]; then
    RECORDING_DIR="$CUSTOM_PATH"
else
    RECORDING_DIR="$HOME/Videos"
fi

set_recording_state() {
    local state=$1
    local STATE_FILE="${XDG_STATE_HOME:-$HOME/.local/state}/mrx/states.json"
    mkdir -p "$(dirname "$STATE_FILE")"
    local tmp=$(mktemp)
    jq ".record.enable = $state" "$STATE_FILE" > "$tmp" && mv "$tmp" "$STATE_FILE"
}

getdate() {
    date '+%Y-%m-%d_%H.%M.%S'
}
default_sink_monitor() {
    local sink
    sink="$(pactl get-default-sink 2>/dev/null)"
    [[ -n "$sink" ]] && echo "${sink}.monitor"
}

default_microphone() {
    local source
    source="$(pactl get-default-source 2>/dev/null)"
    if [[ -n "$source" && "$source" != *.monitor ]]; then
        echo "$source"
    fi
}

MIX_MODULES=()
cleanup_audio() {
    for module in "${MIX_MODULES[@]}"; do
        pactl unload-module "$module" >/dev/null 2>&1
    done
    MIX_MODULES=()
}

AUDIO_SOURCE=""
build_audio_source() {
    local want_system=$1 want_mic=$2 system_source mic_source
    AUDIO_SOURCE=""
    system_source=""
    mic_source=""
    [[ $want_system -eq 1 ]] && system_source="$(default_sink_monitor)"
    if [[ $want_mic -eq 1 ]]; then
        mic_source="$(default_microphone)"
        if [[ -z "$mic_source" ]]; then
            notify-send "No microphone found" "Recording without microphone" -a 'Recorder' & disown
        fi
    fi

    if [[ -n "$system_source" && -n "$mic_source" ]]; then
        local mix="qs_record_mix" module
        module=$(pactl load-module module-null-sink sink_name="$mix" sink_properties=device.description=QuickshellRecordMix) || return
        MIX_MODULES+=("$module")
        module=$(pactl load-module module-loopback source="$system_source" sink="$mix" latency_msec=30) && MIX_MODULES+=("$module")
        module=$(pactl load-module module-loopback source="$mic_source" sink="$mix" latency_msec=30) && MIX_MODULES+=("$module")
        AUDIO_SOURCE="${mix}.monitor"
    elif [[ -n "$system_source" ]]; then
        AUDIO_SOURCE="$system_source"
    elif [[ -n "$mic_source" ]]; then
        AUDIO_SOURCE="$mic_source"
    fi
}

toggle_microphone() {
    local mic
    mic="$(default_microphone)"
    if [[ -z "$mic" ]]; then
        notify-send "No microphone found" "Nothing to mute" -a 'Recorder' & disown
        exit 1
    fi
    pactl set-source-mute "$mic" toggle
    if pactl get-source-mute "$mic" | grep -q yes; then
        notify-send "Microphone muted" -a 'Recorder' & disown
    else
        notify-send "Microphone on" -a 'Recorder' & disown
    fi
}

detect_compositor() { mrx_detect_compositor; }
getactivemonitor() { mrx_get_active_monitor; }

mkdir -p "$RECORDING_DIR"
cd "$RECORDING_DIR" || exit

ARGS=("$@")
MANUAL_REGION=""
SOUND_FLAG=0
MIC_FLAG=0
FULLSCREEN_FLAG=0
for ((i=0;i<${#ARGS[@]};i++)); do
    if [[ "${ARGS[i]}" == "--region" ]]; then
        if (( i+1 < ${#ARGS[@]} )); then
            MANUAL_REGION="${ARGS[i+1]}"
        else
            notify-send "Recording cancelled" "No region specified for --region" -a 'Recorder' & disown
            exit 1
        fi
    elif [[ "${ARGS[i]}" == "--sound" ]]; then
        SOUND_FLAG=1
    elif [[ "${ARGS[i]}" == "--mic" ]]; then
        MIC_FLAG=1
    elif [[ "${ARGS[i]}" == "--fullscreen" ]]; then
        FULLSCREEN_FLAG=1
    elif [[ "${ARGS[i]}" == "--toggle-mic" ]]; then
        toggle_microphone
        exit 0
    fi
done

COMPOSITOR="$(detect_compositor)"
if { ! mrx_is_x11_session "$COMPOSITOR" && pgrep -x wf-recorder >/dev/null; } || { mrx_is_x11_session "$COMPOSITOR" && pgrep -f 'ffmpeg.*x11grab' >/dev/null; }; then
    notify-send "Recording Stopped" "Stopped" -a 'Recorder' &
    if mrx_is_x11_session "$COMPOSITOR"; then pkill -f 'ffmpeg.*x11grab' || true; else pkill wf-recorder || true; fi
    set_recording_state false
else
    if [[ $FULLSCREEN_FLAG -eq 0 && -z "$MANUAL_REGION" ]] && ! mrx_is_x11_session "$COMPOSITOR"; then
        if ! region="$(slurp 2>&1)"; then
            notify-send "Recording cancelled" "Selection was cancelled" -a 'Recorder' & disown
            exit 1
        fi
    fi

    trap cleanup_audio EXIT
    build_audio_source "$SOUND_FLAG" "$MIC_FLAG"
    OUTPUT_FILE="./recording_$(getdate).mp4"
    if mrx_is_x11_session "$COMPOSITOR"; then
        if [[ $FULLSCREEN_FLAG -eq 1 ]]; then
            read -r width height < <(mrx_get_display_geometry)
            x=0; y=0
        elif [[ -n "$MANUAL_REGION" ]]; then
            read -r x y width height < <(printf '%s\n' "$MANUAL_REGION" | sed -E 's/,/ /; s/([0-9]+)x([0-9]+)/\1 \2/')
        else
            if ! read -r x y width height < <(slop -f '%x %y %w %h'); then
                notify-send "Recording cancelled" "Selection was cancelled" -a 'Recorder' & disown
                exit 1
            fi
        fi
        RECORDER_ARGS=(-f x11grab -framerate 30 -video_size "${width}x${height}" -i "${DISPLAY:-:0}+${x},${y}" -pix_fmt yuv420p -c:v libx264 -preset veryfast -crf 23)
        if [[ -n "$AUDIO_SOURCE" ]]; then RECORDER_ARGS+=(-f pulse -i "$AUDIO_SOURCE" -c:a aac); fi
        RECORDER_ARGS+=(-y "$OUTPUT_FILE")
        notify-send "Starting recording" "$(basename "$OUTPUT_FILE")" -a 'Recorder' & disown
        set_recording_state true
        ffmpeg "${RECORDER_ARGS[@]}"
        set_recording_state false
    else
        RECORDER_ARGS=(--pixel-format yuv420p -f "$OUTPUT_FILE")
        if [[ $FULLSCREEN_FLAG -eq 1 ]]; then
            RECORDER_ARGS+=(-o "$(getactivemonitor)")
        else
            RECORDER_ARGS+=(--geometry "$region")
        fi
        if [[ -n "$AUDIO_SOURCE" ]]; then RECORDER_ARGS+=("--audio=$AUDIO_SOURCE"); fi
        notify-send "Starting recording" "$(basename "$OUTPUT_FILE")" -a 'Recorder' & disown
        set_recording_state true
        wf-recorder "${RECORDER_ARGS[@]}"
        set_recording_state false
    fi
fi
