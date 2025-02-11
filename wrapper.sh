#!/bin/bash
# Print the current working directory and environment variables before running the Rust program

# Run the Rust program and capture the output
 { output=$(gits "$@" | tee /dev/fd/3 | grep  "^cd" | cut -d' ' -f2-); } 3>&1
    # Extract the path from the output and change the directory
if [ -n "$output" ]; then
    cd "$output"
fi

