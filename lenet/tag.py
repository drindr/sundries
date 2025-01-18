import glob
import cv2 as cv
import os
from pathlib import Path
if __name__ == '__main__':
    img_file = glob.glob('<data dir>/*.png')
    print(len(img_file))
    for p in img_file:
        img = cv.imread(p)
        cv.imshow('win', img)
        key = cv.waitKey(0)
        file_name = Path(p).stem
        if key == ord('1'):
            os.rename(p, f'<data dir>/<tag 1>/{file_name}.png')
        elif key == ord('2'):
            os.rename(p, f'<data dir>/<tag 2>/{file_name}.png')
        elif key == ord('q'):
            break
