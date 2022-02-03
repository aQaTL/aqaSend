#!/bin/bash

export PATH=$PATH:/node-v16.13.2-linux-x64/bin/
source $HOME/.cargo/env
npm --version
cargo --version
rustc --version

cd /aqaSend

cd aqa_send_web 
npm install
cd .. 
cargo build --release -p aqa_send_web_server
cargo build --release -p aqa_send 
