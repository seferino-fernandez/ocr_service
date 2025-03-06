# Scripts

## download-tessdata.sh

Downloads the Tesseract data files to the specified directory. By default, files are downloaded to the `tesseract` directory, but you can change this by setting the `TESSDATA_PATH` environment variable.

**View script usage:**

```shell
./scripts/download-tessdata.sh --help
```

**Download all files:**

```shell
# Download to default location (./tesseract)
./scripts/download-tessdata.sh --all

# Download to custom location
TESSDATA_PATH=/path/to/custom/location ./scripts/download-tessdata.sh --all
```

**Download specific files:**

```shell
# Download to default location (./tesseract)
./scripts/download-tessdata.sh eng chi_sim chi_tra

# Download to custom location
TESSDATA_PATH=/path/to/custom/location ./scripts/download-tessdata.sh eng chi_sim chi_tra
```

**List all available languages:**

```shell
./scripts/download-tessdata.sh --list
```
