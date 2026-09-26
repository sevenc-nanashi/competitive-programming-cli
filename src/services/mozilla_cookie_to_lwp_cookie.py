import sys
import http.cookiejar

base = sys.argv[1]
convert_to = sys.argv[2]

lwp_cookie = http.cookiejar.LWPCookieJar(convert_to)
if base != "<none>":
    mozilla_cookie = http.cookiejar.MozillaCookieJar(base)
    mozilla_cookie.load(ignore_discard=True, ignore_expires=True)
    for cookie in mozilla_cookie:
        lwp_cookie.set_cookie(cookie)
lwp_cookie.save(ignore_discard=True, ignore_expires=True)
