"""Trace the two half-symbols out of the old app icon into SVG paths.
Output: unit coordinates centred on the symbol centre, Veridican radius = 1,
each path being the RIGHT half of its symbol."""
import sys, json
import numpy as np, cv2
from PIL import Image
im = np.asarray(Image.open(sys.argv[1]).convert('RGB')).astype(int)
r, g, b = im[..., 0], im[..., 1], im[..., 2]
masks = {
    'V': (r < 90) & (g < 80) & (b < 80),
    'C': (abs(r - g) < 16) & (abs(g - b) < 16) & (r > 185),
}
CX, CY, R, UP = 258.5, 256.0, 227.5, 6
out = {}
for name, m in masks.items():
    m8 = (m * 255).astype(np.uint8)
    n, lab, stats, _ = cv2.connectedComponentsWithStats(m8, 8)
    for i in range(1, n):                       # drop specks
        if stats[i, cv2.CC_STAT_AREA] < 40:
            m8[lab == i] = 0
    big = cv2.resize(m8, None, fx=UP, fy=UP, interpolation=cv2.INTER_CUBIC)
    big = cv2.GaussianBlur(big, (0, 0), UP * 0.6)
    _, big = cv2.threshold(big, 127, 255, cv2.THRESH_BINARY)
    cs, _ = cv2.findContours(big, cv2.RETR_CCOMP, cv2.CHAIN_APPROX_NONE)
    d, pts = '', 0
    for c in cs:
        if cv2.contourArea(c) < 30 * UP * UP:
            continue
        a = cv2.approxPolyDP(c, 0.35 * UP, True)[:, 0, :].astype(float) / UP
        xs = (a[:, 0] - CX) / R
        ys = (a[:, 1] - CY) / R
        if name == 'V':
            xs = -xs                             # the icon shows its left half
        d += 'M' + 'L'.join(f'{x:.3f},{y:.3f}' for x, y in zip(xs, ys)) + 'Z'
        pts += len(a)
    out[name] = d
    print(name, 'points', pts, 'chars', len(d), file=sys.stderr)
json.dump(out, open(sys.argv[2], 'w'))
