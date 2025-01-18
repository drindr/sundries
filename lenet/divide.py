import glob
import random
import os
from pathlib import Path
if __name__ == '__main__':
    files = glob.glob('<data dir>/<tag>/*.png')
    rate = 0.8
    l = len(files)
    train_num = int(rate * len(files))

    train_file = random.sample(files, k = int(rate * len(files)))
    for file_name in files:
        path = Path(file_name).stem
        if file_name in train_file:
            os.rename(file_name, f'<data dir>/<tag>/train/{path}.png')
        else:
            os.rename(file_name, f'<data dir>/<tag>/test/{path}.png')
    
