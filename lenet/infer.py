import torch
from torchvision import transforms
import cv2 as cv
import numpy as np

class FixedRatioResize:
    def __init__(self, size):
        self.size = size
        
    def __call__(self, img):
        h, w = img.shape[1:]
        if w > h:
            ratio = self.size / w
            new_h = int(ratio * h)
            new_w = self.size
        else:
            ratio = self.size / h
            new_h = self.size
            new_w = int(ratio * w)
        return transforms.functional.resize(img, (new_w, new_h))

class PadToSquare:
    def __init__(self, size):
        self.size = size
        
    def __call__(self, img):
        h, w = img.shape[1:]
        pad_h = max(0, self.size - h)
        pad_w = max(0, self.size - w)
        pad_h_t = int(pad_h / 2)
        pad_h_b = pad_h - pad_h_t

        pad_w_l = int(pad_w / 2)
        pad_w_r = pad_w - pad_w_l
        ret =  transforms.functional.pad(img, (pad_w_l, pad_h_t, pad_w_r, pad_h_b))
        # print(f'new:  {pad_w} {w}, {pad_h} {h}, {ret.shape}')
        return ret

def preprocess(image):
    #image = cv.cvtColor(image, cv.COLOR_BGR2GRAY)

    transform = transforms.Compose([transforms.ToTensor(), FixedRatioResize(32), PadToSquare(32),transforms.Normalize(mean = (0.1307, ), std = (0.3081, ))])
    return torch.tensor(transform(image))

def infer(net, image):
    image = preprocess(image)
    print(image.shape)
    batch = torch.stack([image], dim = 0)
    return net(batch)

if __name__ == '__main__':
    import model
    net = model.LeNet()
    net.load_state_dict(torch.load('./<model file>', weights_only=True))
    img = cv.imread('<test image>.png')
    print(infer(net, img))
