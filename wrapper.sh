#!/bin/bash
# Print the current working directory and environment variables before running the Rust program

# Run the Rust program and capture the output
output=$(gits "$@")
if echo "$output" | grep -q "^cd "; then
    # Extract the path from the output and change the directory
    p=$(echo "$output" | grep "^cd " | cut -d' ' -f2-)wrap
    cd "$p"
    echo "$output"
else
    echo "$output"
fi

