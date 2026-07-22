import sys
sys.path.insert(0, '.')
import apeireth.v1060_asi_orchestrator as m
b = m.ASIOrchestratorBridge()
import inspect
attrs = [x for x in dir(b) if not x.startswith('_')]
print('attrs:', attrs)
print('---')
for x in attrs:
    try:
        attr = getattr(b, x)
        if callable(attr):
            sig = inspect.signature(attr)
            if not sig.parameters:
                print(f'{x}() = {attr()}')
            else:
                print(f'{x}{sig}')
        else:
            print(f'{x} = {attr}')
    except Exception as e:
        print(f'{x} ERR: {e}')