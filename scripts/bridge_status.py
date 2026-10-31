#!/usr/bin/env python3
"""Query Bridge's sanitized reporter snapshot without opening/stopping its CLI."""
import argparse
import json
import math
from pathlib import Path
import stat
import time


def query(path, now):
    try:
        metadata=path.lstat()
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 8192:
            raise ValueError()
        with path.open('rb') as source:
            data=source.read(8193)
        if len(data)>8192:raise ValueError()
        value=json.loads(data)
        if type(value['schema']) is not int or value['schema']!=1 or value['phase'] not in ('starting','unknown','syncing','synced','interrupted','stopped'):
            raise ValueError()
        observed=value['observed_at'];event=value.get('event_at')
        if type(observed) is not int or observed<0 or (event is not None and (type(event) is not int or not 0<=event<=observed)):
            raise ValueError()
        if value['phase'] in ('syncing','synced','interrupted') and event is None:
            raise ValueError()
        for name in ('percent','elapsed_seconds','remaining_seconds'):
            number=value.get(name)
            if number is not None and (type(number) not in (int,float) or not math.isfinite(number) or number<0):
                raise ValueError()
        if value.get('percent') is not None and value['percent']>100:
            raise ValueError()
        if value['phase']!='syncing' and (value.get('percent') is not None or value.get('remaining_seconds') is not None):
            raise ValueError()
        if value['observed_at']>now+5 or now-value['observed_at']>20:
            return 'Bridge status is stale; check the container and reporter.',2
        phase={'starting':'starting','unknown':'running; sync progress not yet reported','syncing':'syncing mailbox','synced':'reported sync finished','interrupted':'sync interrupted; waiting for retry','stopped':'stopped'}[value['phase']]
        text='Bridge: '+phase
        percent=value.get('percent')
        if value['phase']=='syncing' and isinstance(percent,(int,float)) and 0<=percent<=100:
            text+=f' · last reported {percent:.1f}%'
        if isinstance(event,int) and event<=now:
            text+=f' · report age {int(now-event)} seconds'
        return text,0
    except (OSError,ValueError,KeyError,TypeError,OverflowError,RecursionError):
        return 'Bridge status is unavailable; check the container and reporter.',2


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('snapshot',type=Path)
    args=p.parse_args()
    text,code=query(args.snapshot,time.time())
    print(text)
    raise SystemExit(code)
