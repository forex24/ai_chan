import pandas as pd

from Common.CEnum import DATA_FIELD, KL_TYPE
from Common.ChanException import CChanException, ErrCode
from Common.CTime import CTime
from Common.func_util import str2float
from KLine.KLine_Unit import CKLine_Unit

from .CommonStockAPI import CCommonStockApi


def create_item_dict(data, column_name):
    for i in range(len(data)):
        data[i] = parse_time_column(data[i]) if column_name[i] == DATA_FIELD.FIELD_TIME else float(data[i])

    return dict(zip(column_name, data))

def parse_time_column(inp):
    # 如果inp是Timestamp类型调用parse_time_column_by_datetime
    if isinstance(inp, pd.Timestamp):
        return parse_time_column_by_datetime(inp)
    else:
        return parse_time_column_by_str(inp)
        
def parse_time_column_by_datetime(inp):
    #print("time:",inp)
    return CTime(inp.year, inp.month, inp.day, inp.hour, inp.minute)


def parse_time_column_by_str(inp):
    # 20210902113000000
    # 2021-09-13
    if len(inp) == 10:
        year = int(inp[:4])
        month = int(inp[5:7])
        day = int(inp[8:10])
        hour = minute = 0
    elif len(inp) == 17:
        year = int(inp[:4])
        month = int(inp[4:6])
        day = int(inp[6:8])
        hour = int(inp[8:10])
        minute = int(inp[10:12])
    elif len(inp) == 19:
        year = int(inp[:4])
        month = int(inp[5:7])
        day = int(inp[8:10])
        hour = int(inp[11:13])
        minute = int(inp[14:16])
    else:
        raise Exception(f"unknown time column from csv:{inp}")
    return CTime(year, month, day, hour, minute)


class DATAFRAME_API(CCommonStockApi):
    def __init__(self, code, k_type=KL_TYPE.K_DAY, begin_date=None, end_date=None, autype=None):
        self.columns = [
            DATA_FIELD.FIELD_TIME,
            DATA_FIELD.FIELD_OPEN,
            DATA_FIELD.FIELD_HIGH,
            DATA_FIELD.FIELD_LOW,
            DATA_FIELD.FIELD_CLOSE,
            DATA_FIELD.FIELD_VOLUME,
            # DATA_FIELD.FIELD_TURNOVER,
            # DATA_FIELD.FIELD_TURNRATE,
        ]  # 每一列字段
        self.df = code  # 传入的DataFrame
        super(DATAFRAME_API, self).__init__(code, k_type, begin_date, end_date, autype)
        self.begin_date = begin_date #pd.to_datetime(begin_date) if begin_date else None
        self.end_date = end_date #pd.to_datetime(end_date) if end_date else None
        
    def get_kl_data(self):
        if self.df is None:
            raise CChanException("DataFrame is not provided", ErrCode.SRC_DATA_NOT_FOUND)

        if self.begin_date is not None and self.end_date is not None:
            self.df = self.df[(self.df[DATA_FIELD.FIELD_TIME] >= self.begin_date) & (self.df[DATA_FIELD.FIELD_TIME] < self.end_date)]
        elif self.begin_date is not None:
            self.df = self.df[self.df[DATA_FIELD.FIELD_TIME] >= self.begin_date]
        elif self.end_date is not None:
            self.df = self.df[self.df[DATA_FIELD.FIELD_TIME] < self.end_date]
        
        self.df = self.df[self.columns]
        
        for _, row in self.df.iterrows():
            data = row.tolist()
            yield CKLine_Unit(create_item_dict(data, self.columns))

    def SetBasciInfo(self):
        pass

    @classmethod
    def do_init(cls):
        pass

    @classmethod
    def do_close(cls):
        pass