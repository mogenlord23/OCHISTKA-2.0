import sys,glob,os
from PIL import Image
d=sys.argv[1]; out=sys.argv[2]
fs=sorted(glob.glob(d+'/*.png')); fs=[f for f in fs if 'x' not in os.path.basename(f).split('-')[-1]]
for k in range(0,len(fs),4):
    sh=Image.new('RGB',(1580,1076))
    for i,f in enumerate(fs[k:k+4]): sh.paste(Image.open(f).convert('RGB').resize((790,538)),((i%2)*790,(i//2)*538))
    sh.save(f'{out}{k//4}.png')
