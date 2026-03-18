import sqlite3, json, os
p = os.path.join(os.environ.get('APPDATA'), 'bili-sync', 'data.sqlite')
if not os.path.exists(p):
    p = os.path.join(os.getcwd(), 'data.sqlite')
conn = sqlite3.connect(p)
cur = conn.cursor()
cur.execute('SELECT data FROM config WHERE id = 1')
row = cur.fetchone()
if not row:
    print('no config row')
else:
    data = json.loads(row[0])
    print(json.dumps(data, ensure_ascii=False, indent=2))
conn.close()
