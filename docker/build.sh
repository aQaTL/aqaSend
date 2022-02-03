#!/bin/bash
docker run --mount type=bind,source="$(pwd)"/..,target=/aqaSend --rm aqasend
