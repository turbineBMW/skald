"""One-time login. Opens the Amazon login page in your browser; paste the final
redirect URL (the one that 'fails' to load, starting with https://www.amazon.com/ap/maplanding) back here."""
import audible, sys
locale = sys.argv[1] if len(sys.argv) > 1 else "us"
auth = audible.Authenticator.from_login_external(locale=locale)
auth.to_file("auth.json")
print("saved auth.json for locale", locale)
