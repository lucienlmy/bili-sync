import os
import sqlite3
import json

appdata = os.environ.get('APPDATA')
paths = []
if appdata:
    paths.append(os.path.join(appdata, 'bili-sync', 'data.sqlite'))
paths.append(os.path.join(os.getcwd(), 'data.sqlite'))

db_path = None
for p in paths:
    if os.path.exists(p):
        db_path = p
        break

if not db_path:
    print(json.dumps({"error": "database not found", "tried": paths}, ensure_ascii=False))
    exit(0)

try:
    conn = sqlite3.connect(db_path)
    cur = conn.cursor()
    def safe_exec(q):
        try:
            cur.execute(q)
            return cur.fetchall()
        except Exception as e:
            return str(e)

    result = {"db_path": db_path}
    # counts
    result['counts'] = {}
    for t in ('video','page'):
        try:
            cur.execute(f"SELECT COUNT(*) FROM {t}")
            result['counts'][t] = cur.fetchone()[0]
        except Exception as e:
            result['counts'][t] = str(e)
    # sqlite_sequence
    try:
        cur.execute("SELECT name, seq FROM sqlite_sequence WHERE name IN ('video','page')")
        result['sqlite_sequence'] = {name: seq for name, seq in cur.fetchall()}
    except Exception as e:
        result['sqlite_sequence'] = str(e)
    # latest_row_at for sources
    result['latest_row_at'] = {}
    sources = ['favorite','collection','submission','watch_later']
    for s in sources:
        try:
            cur.execute(f"SELECT id, latest_row_at FROM {s}")
            rows = cur.fetchall()
            # convert to list of tuples
            result['latest_row_at'][s] = rows
        except Exception as e:
            result['latest_row_at'][s] = str(e)
    # sample recent videos
    try:
        cur.execute("SELECT id, bvid, name, path, download_status, should_download, valid, created_at FROM video ORDER BY id DESC LIMIT 10")
        result['recent_videos'] = cur.fetchall()
    except Exception as e:
        result['recent_videos'] = str(e)

    print(json.dumps(result, ensure_ascii=False, indent=2))
finally:
    try:
        conn.close()
    except:
        pass
