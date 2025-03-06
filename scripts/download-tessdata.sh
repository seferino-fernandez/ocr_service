#!/bin/bash
#
# Download Tesseract language data files from GitHub.

set -e
set -o pipefail

err() {
    echo "[$(date +'%Y-%m-%dT%H:%M:%S%z')]: $*" >&2
}

# Get the download directory from TESSDATA_PATH environment variable or default to ./tesseract
DOWNLOAD_DIR="${TESSDATA_PATH:-./tesseract}"

# Check if download directory exists, create it if not
if ! mkdir -p "$DOWNLOAD_DIR"; then
    err "Error creating directory '$DOWNLOAD_DIR'. Exiting."
    exit 1
fi

# Array of all language codes supported by Tesseract - https://github.com/tesseract-ocr/tessdata
declare -a languages=(
    afr amh ara asm aze aze_cyrl bel ben bod bos bre bul cat ceb ces
    chi_sim chi_sim_vert chi_tra chi_tra_vert chr cos cym dan dan_frak deu
    deu_frak deu_latf div dzo ell eng enm epo equ est eus fao fas fil fin
    fra frm fry gla gle glg grc guj hat heb hin hrv hun hye iku ind isl
    ita ita_old jav jpn jpn_vert kan kat kat_old kaz khm kir kmr kor
    kor_vert lao lat lav lit ltz mal mar mkd mlt mon mri msa mya nep nld
    nor oci ori osd pan pol por pus que ron rus san sin slk slk_frak slv
    snd spa spa_old sqi srp srp_latn sun swa swe syr tam tat tel tgk tgl
    tha tir ton tur uig ukr urd uzb uzb_cyrl vie yid yor
)

readonly DEFAULT_TESSDATA_URL="https://github.com/tesseract-ocr/tessdata/raw/refs/heads/main"
readonly FAST_TESSDATA_URL="https://github.com/tesseract-ocr/tessdata_fast/raw/refs/heads/main"
readonly BEST_TESSDATA_URL="https://github.com/tesseract-ocr/tessdata_best/raw/refs/heads/main"

# Function to download a single language traineddata file
download_lang() {
    local lang="$1"
    local url="$DEFAULT_TESSDATA_URL/$lang.traineddata"
    if wget -qO "$DOWNLOAD_DIR/$lang.traineddata" "$url"; then
        echo "Downloaded $lang.traineddata to $DOWNLOAD_DIR"
    else
        err "Failed to download $lang.traineddata"
    fi
}

help_command() {
    echo "Usage: $0 [--all] [--list] [--help] [eng chi_sim ...]"
    echo "Downloads Tesseract language data files."
    echo "  --all: Downloads all available languages."
    echo "         eng chi_sim chi_tra ...: Downloads the specified languages (English, Chinese Simplified, Chinese Traditional, etc)."
    echo "  --list: Lists all available languages."
    echo "  --help: Displays this help message."
    echo ""
    echo "Note: Files will be downloaded to the directory specified by TESSDATA_PATH environment variable"
    echo "      (defaults to ./tesseract if not set)."
}

list_languages() {
    echo "Available languages:"
    for lang in "${languages[@]}"; do
        echo "  $lang"
    done
}

main() {
    if [[ "$1" == "--all" ]]; then
        echo "Downloading all available languages..."
        for lang in "${languages[@]}"; do
            download_lang "$lang"
        done
    elif [[ "$1" == "--list" ]]; then
        list_languages
    elif [[ "$1" == "--help" ]]; then
        help_command
    elif [[ -z "$1" ]]; then
        help_command
    else
        # Loop through provided language codes, first validating the language code exists, then downloading the file
        for lang in "$@"; do
            language_valid=false
            for valid_lang in "${languages[@]}"; do
                if [[ "$lang" == "$valid_lang" ]]; then
                    language_valid=true
                    break
                fi
            done

            if ! $language_valid; then
                echo "Invalid language code: $lang. Skipping..."
                continue
            fi
            download_lang "$lang"
        done
    fi
}

main "$@"
