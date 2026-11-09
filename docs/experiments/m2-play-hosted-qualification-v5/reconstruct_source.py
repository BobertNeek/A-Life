"""Pure, strict text-only unified patch application. Never runs a compiler."""
import pathlib
import re

def apply(data, read):
    data.decode('utf-8', 'strict')
    assert b'\0' not in data and b'GIT binary patch' not in data
    lines = data.splitlines(keepends=True)
    result = {}
    i = 0
    while i < len(lines):
        if not lines[i].startswith(b'--- '):
            assert not lines[i].startswith((b'old mode', b'new mode', b'rename ', b'deleted file'))
            i += 1
            continue
        old_name = lines[i][4:].rstrip(b'\n').decode()
        i += 1
        assert lines[i].startswith(b'+++ ')
        name = lines[i][4:].rstrip(b'\n').decode()
        assert name.startswith('b/')
        path = name[2:]
        assert re.fullmatch(r'[A-Za-z0-9_.\-/]+', path)
        assert not pathlib.PurePosixPath(path).is_absolute()
        assert '..' not in pathlib.PurePosixPath(path).parts and path not in result
        assert old_name in ('a/' + path, '/dev/null')
        old = read(path)
        assert (old is None) == (old_name == '/dev/null')
        original = (old or b'').splitlines(keepends=True)
        out = []
        cursor = 0
        i += 1
        hunks = 0
        while i < len(lines) and lines[i].startswith(b'@@ '):
            h = re.match(rb'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@', lines[i])
            assert h
            old_count, new_count = int(h[2] or 1), int(h[4] or 1)
            start = int(h[1]) - (1 if old_count else 0)
            assert cursor <= start <= len(original)
            out.extend(original[cursor:start]); cursor = start
            i += 1
            consumed = added = 0
            while consumed < old_count or added < new_count:
                line = lines[i]
                assert line[:1] in (b' ', b'-', b'+')
                body = line[1:]
                i += 1
                if i < len(lines) and lines[i].startswith(b'\\ No newline'):
                    body = body.removesuffix(b'\n'); i += 1
                if line[:1] in (b' ', b'-'):
                    assert cursor < len(original) and original[cursor] == body
                    cursor += 1; consumed += 1
                if line[:1] in (b' ', b'+'):
                    out.append(body); added += 1
            assert (consumed, added) == (old_count, new_count)
            hunks += 1
        assert hunks
        out.extend(original[cursor:])
        result[path] = b''.join(out)
    assert result
    return result


import hashlib

def verify(mapping, rows):
    expected={r['path']:r for r in rows}
    assert len(expected)==len(rows) and set(mapping)==set(expected)
    for name,b in mapping.items():
        assert len(b)==expected[name]['bytes'] and hashlib.sha256(b).hexdigest()==expected[name]['sha256']

def reconstruct_base(main, config, read_blob, base_rows):
    verify(main,config['checkout_files'])
    current=dict(main)
    for step in config['steps']:
        chunks=[]
        for row in step['parts']:
            b=read_blob(row['path'])
            assert len(b)==row['bytes'] and hashlib.sha256(b).hexdigest()==row['sha256']
            chunks.append(b)
        patch=b''.join(chunks)
        assert len(patch)==step['patch_bytes'] and hashlib.sha256(patch).hexdigest()==step['patch_sha256']
        changed=apply(patch,current.get)
        verify(changed,step['outputs'])
        current.update(changed)
    result={r['path']:current[r['path']] for r in base_rows}
    verify(result,base_rows)
    return result
