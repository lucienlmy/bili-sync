import sqlite3, json, os, sys

p = os.path.join(os.environ.get('APPDATA', ''), 'bili-sync', 'data.sqlite')
if not os.path.exists(p):
    print(f"NOFILE:{p}")
    sys.exit(2)
try:
    c = sqlite3.connect(p)
    row = c.execute('select data from config where id=1').fetchone()
    if not row:
        print('NODATA')
        sys.exit(3)
    data = json.loads(row[0])
    token = data.get('auth_token')
    if not token:
        print('NOTOKEN')
        sys.exit(4)
    print(token)
except Exception as e:
    print('ERROR:'+str(e))
    sys.exit(1)
