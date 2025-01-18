from torch import optim
from torchvision.datasets import mnist
from torch.utils.data import Dataset, DataLoader
import model
from infer import *
from torchvision import transforms
from torch import nn
import torch
import numpy as np
import glob
import cv2 as cv

class RedDataset(Dataset):
    def __init__(self, train = True):
        self.train_file = []
        if train is True:
            # load train data
            self.train_file = self.train_file + list(map(lambda f: (f, 0), glob.glob('<data dir>/<tag 0>/train/*.png')))
            self.train_file = self.train_file + list(map(lambda f: (f, 1), glob.glob('<data dir>/<tag 1>/train/*.png')))
        else:
            # load test data
            self.train_file = self.train_file + list(map(lambda f: (f, 0), glob.glob('<data dir>/<tag 0>/test/*.png')))
            self.train_file = self.train_file + list(map(lambda f: (f, 1), glob.glob('<data dir>/<tag 1>/test/*.png')))

        self.transform = transforms.Compose([transforms.ToTensor(), FixedRatioResize(32), PadToSquare(32),transforms.Normalize(mean = (0.1307, ), std = (0.3081, ))])
    def __len__(self):
        return len(self.train_file)
    def __getitem__(self, idx):
        file_path, label = self.train_file[idx]
        image = cv.imread(file_path)
        #image = cv.cvtColor(image, cv.COLOR_BGR2GRAY)
        image = self.transform(image)
        # print(image.shape)
        return image, label

if __name__ == '__main__':
    net = model.LeNet()
    batch_size = 256
    print(net)
    train_data = RedDataset()
    test_data = RedDataset(train = False)

    train_loader = DataLoader(train_data, batch_size = 64, shuffle = True)
    test_loader = DataLoader(test_data, batch_size = 64, shuffle = True)
    optimizer = optim.SGD(net.parameters(), lr = 0.01)
    optimizer.zero_grad()
    criterion = nn.MSELoss()
    for epoch in range(10):
        for train_x, train_label in train_loader:
            output = net(train_x)
            train_label = nn.functional.one_hot(train_label, num_classes = 3).float()
            loss = criterion(output, train_label)
            loss.backward()
            optimizer.step()
            print(loss)

    correct_num = 0
    sample_num = 0
    for test_x, test_label in test_loader:
        output = net(test_x)
        output = torch.argmax(output, dim = -1)
        correct = output == test_label
        correct_num += np.sum(correct.numpy(), axis = -1)
        sample_num += correct.shape[0]

    print(correct_num / sample_num)

    torch.save(net.state_dict(), '<model name>')
