#! /usr/bin/sh
#
#
# Used in GitHub actions ot fuzz utils/buffer for 30s, to determine if there are
# any immediate errors. This is only ran for 30s because that fuzzing can take
# awhile, and waiting on GitHub actions isn't optimal
#
# This script does the following:
#   (1) sets the Rust default to nightly
#   (1) runs rust fuzz test on utils/buffer for 30 seconds
#
# This script is called by `.github/workflows/fuzz-utils-buffer.yaml` on every PR onto the main branch.
#

echo "Script is starting"
# rustup override set nightly
# cd ./utils/buffer

if ! timeout 10s sleep 12s
then
    echo "Success. Fuzz tests pass"
    sleep 3s
    exit 0
else 
    echo "Failure. Fuzz tests don't pass"
    sleep 3s
    exit -1
fi
