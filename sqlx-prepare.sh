#!/bin/sh

cargo sqlx prepare -- --lib
mv sqlx-data.json sqlx-data.lib.json

# https://stackoverflow.com/questions/19529688/how-to-merge-2-json-objects-from-2-files-using-jq
cat \
    sqlx-data.lib.json \
| jq -s add > sqlx-data.json

rm \
    sqlx-data.lib.json \
    sqlx-data.file.json \