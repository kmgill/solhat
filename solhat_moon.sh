#!/bin/bash

if [ $# -lt 1 ]; then
    echo "USAGE: run.sh </dataroot>"
    exit 1
fi

MASKROOT=~/repos/solar_ha_processing/masks/
DATAROOT=$1

if [ $# -eq 2 ]; then
    VERSION=_$2
else
    VERSION=""
fi

MOON_ROOT=Moon
MOON_DARK_ROOT=Moon_-_Dark

# location.sh should be an executable script setting the variables
# LOC_LATITUDE and LOC_LONGITUDE for the location the observations
# were made.
source location.sh

SOLHAT_BIN=solhat
export MARS_OUTPUT_FORMAT=tif
STUMP_LOG_AT_LEVEL=info

MOON_MAX_SCALE=95

CROP_WIDTH=2400
CROP_HEIGHT=2400
check_file=`ls -1 $DATAROOT/$MOON_ROOT/*/*ser | head -n 1`
BIT_DEPTH=`$SOLHAT_BIN ser-info -i $check_file | grep "Pixel Depth" | cut -d ' ' -f 3`

FRAME_LIMIT=1000

DATA_TS=`ls $DATAROOT/$MOON_ROOT/ | tail -n 1`

MOON_THRESH=15000
MOON_SIGMA_MIN=0.0
MOON_SIGMA_MAX=2000.0
MOON_TOP_PCT=10
DRIZZLE_SCALE=1.5

echo Data Root: $DATAROOT
echo Moon Root: $DATAROOT/$MOON_ROOT
echo Moon Dark Root: $DATAROOT/$MOON_DARK_ROOT
echo Expected Bit Depth: $BIT_DEPTH
echo Data Timestamp: $DATA_TS
echo Version Text: $VERSION

echo
echo Output Moon: $DATAROOT/Moon_${DATA_TS}${VERSION}.tif

DARK_FRAME=$DATAROOT/Dark_${DATA_TS}${VERSION}.tif
if [ ! -f $DARK_FRAME ]; then
    echo Creating calibration frames...
    $SOLHAT_BIN -v mean -i $DATAROOT/$MOON_DARK_ROOT/*/*ser -o $DARK_FRAME
    if [ ! -f $DARK_FRAME -o $? -ne 0 ]; then
        echo Error: Failed to generate dark frame
    fi
fi



echo Generating threshold test frame...
$SOLHAT_BIN thresh-test -i $DATAROOT/$MOON_ROOT/*/*ser \
                        -o $DATAROOT/ThreshTest_${DATA_TS}${VERSION}.tif \
                        -t $MOON_THRESH

echo "Starting Moon Processing..."
$SOLHAT_BIN process -i $DATAROOT/$MOON_ROOT/*/Moon*ser \
                    -o $DATAROOT/Moon_RGB_${DATA_TS}${VERSION}.tif \
                    -t $MOON_THRESH \
                    -l $LOC_LATITUDE \
                    -L $LOC_LONGITUDE \
                    -P $MOON_TOP_PCT \
                    -n $FRAME_LIMIT \
                    -S $MOON_SIGMA_MAX \
                    -s $MOON_SIGMA_MIN \
                    -T moon \
                    -u $DRIZZLE_SCALE
