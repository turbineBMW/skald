"""Position-sync probe.
  uv run probe.py                 list library + last positions
  uv run probe.py ASIN            show license/acr + current position for one book
  uv run probe.py ASIN POS_MS     write position via PUT /1.0/lastpositions/{asin}
"""
import audible, json, sys
auth = audible.Authenticator.from_file("auth.json")
with audible.Client(auth=auth) as c:
    if len(sys.argv) == 1:
        lib = c.get("1.0/library", num_results=50, response_groups="product_desc")
        items = lib["items"]
        asins = ",".join(i["asin"] for i in items)
        pos = c.get("1.0/annotations/lastpositions", asins=asins)
        print(json.dumps(pos, indent=1)[:1500], "\n")
        for it in items:
            print(f"  {it['asin']}  {it.get('title')}")
        sys.exit()

    asin = sys.argv[1]
    lic = c.post(f"1.0/content/{asin}/licenserequest", body={
        "consumption_type": "Download", "quality": "High",
        "supported_drm_types": ["Mpeg", "Adrm"],
        "response_groups": "last_position_heard,content_reference",
    })["content_license"]
    acr = lic["acr"]
    print("acr:", acr)
    print("status:", lic["status_code"])
    print("last_position_heard:", lic["content_metadata"].get("last_position_heard"))
    print("format:", lic["content_metadata"].get("content_reference", {}).get("content_format"))

    if len(sys.argv) > 2:
        ms = int(sys.argv[2])
        r = c.put(f"1.0/lastpositions/{asin}", body={"acr": acr, "asin": asin, "position_ms": ms})
        print("\nPUT ->", r)
        print("readback:", c.get("1.0/annotations/lastpositions", asins=asin))
