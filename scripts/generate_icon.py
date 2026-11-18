"""Render the original Zoxryn vector mark to Windows icon sizes. Pillow required."""
from pathlib import Path
from PIL import Image, ImageDraw
root = Path(__file__).resolve().parents[1] / 'src-tauri' / 'icons'
root.mkdir(parents=True, exist_ok=True)
size = 1024
im = Image.new('RGBA', (size,size), (0,0,0,0))
draw = ImageDraw.Draw(im)
draw.rounded_rectangle((20,20,1004,1004), radius=230, fill='#17121f', outline='#624083', width=12)
mark = [(270,290),(810,290),(421,676),(767,676),(695,796),(196,796),(609,410),(222,410)]
draw.polygon(mark, fill='#aa7aef')
draw.polygon([(685,148),(851,194),(804,360),(713,289)], fill='#dac3fa')
im.save(root / 'icon.png')
for n in (32,128,256):
    im.resize((n,n),Image.Resampling.LANCZOS).save(root / f'{n}x{n}.png')
im.save(root / 'icon.ico', sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
print(f'Created original application icons in {root}')
