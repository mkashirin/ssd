#!/bin/bash

DB_FILE="users.db"

if [ -f "$DB_FILE" ]; then
    echo "[-] Removing the old database file $DB_FILE..."
    rm "$DB_FILE"
fi

echo "[*] Compiling and running the seeder..."
cargo run --bin seed

if [ $? -eq 0 ]; then
    echo "[+] The database has been initialized successfully!"
    echo "[+] Now run the server with cargo run"
else
    echo "[!] Error ocurred while initializing the DB."
    exit 1
fi
