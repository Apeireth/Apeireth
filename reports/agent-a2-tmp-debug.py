import yaml
text = open(r'.github\ISSUE_TEMPLATE\feature_request.yml', encoding='utf-8').read()
data = yaml.safe_load(text)
for i, b in enumerate(data['body']):
    if isinstance(b, dict):
        bid = b.get('id', '<no-id>')
        btype = b.get('type', '?')
        if btype == 'checkboxes':
            opts = b.get('attributes', {}).get('options', [])
            print('body[{}] type=checkboxes id={} options_count={}'.format(i, bid, len(opts)))
            for j, o in enumerate(opts):
                req = o.get('required')
                lbl = o.get('label', '')
                print('    opt[{}] required={} label={!r}'.format(j, req, lbl[:30]))
        else:
            print('body[{}] type={} id={}'.format(i, btype, bid))
