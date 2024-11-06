import copy
import os
from typing import Dict, List, Union, overload

import pandas as pd

from Bi.Bi import CBi
from Bi.BiList import CBiList
from BuySellPoint.BSPointList import CBSPointList
from ChanConfig import CChanConfig
from Common.CEnum import KLINE_DIR, SEG_TYPE
from Common.ChanException import CChanException, ErrCode
from Seg.Seg import CSeg
from Seg.SegConfig import CSegConfig
from Seg.SegListComm import CSegListComm
from ZS.ZSList import CZSList

from .KLine import CKLine
from .KLine_Unit import CKLine_Unit


def get_seglist_instance(seg_config: CSegConfig, lv) -> CSegListComm:
    if seg_config.seg_algo == "chan":
        from Seg.SegListChan import CSegListChan
        return CSegListChan(seg_config, lv)
    elif seg_config.seg_algo == "1+1":
        print(f'Please avoid using seg_algo={seg_config.seg_algo} as it is deprecated and no longer maintained.')
        from Seg.SegListDYH import CSegListDYH
        return CSegListDYH(seg_config, lv)
    elif seg_config.seg_algo == "break":
        print(f'Please avoid using seg_algo={seg_config.seg_algo} as it is deprecated and no longer maintained.')
        from Seg.SegListDef import CSegListDef
        return CSegListDef(seg_config, lv)
    else:
        raise CChanException(f"unsupport seg algoright:{seg_config.seg_algo}", ErrCode.PARA_ERROR)


class CKLine_List:
    def __init__(self, kl_type, conf: CChanConfig):
        self.kl_type = kl_type
        self.config = conf
        self.lst: List[CKLine] = []  # K线列表，可递归  元素KLine类型
        self.bi_list = CBiList(bi_conf=conf.bi_conf)
        self.seg_list: CSegListComm[CBi] = get_seglist_instance(seg_config=conf.seg_conf, lv=SEG_TYPE.BI)
        self.segseg_list: CSegListComm[CSeg[CBi]] = get_seglist_instance(seg_config=conf.seg_conf, lv=SEG_TYPE.SEG)

        self.zs_list = CZSList(zs_config=conf.zs_conf)
        self.segzs_list = CZSList(zs_config=conf.zs_conf)

        self.bs_point_lst = CBSPointList[CBi, CBiList](bs_point_config=conf.bs_point_conf)
        self.seg_bs_point_lst = CBSPointList[CSeg, CSegListComm](bs_point_config=conf.seg_bs_point_conf)

        self.metric_model_lst = conf.GetMetricModel()

        self.step_calculation = self.need_cal_step_by_step()

        self.bs_point_history: List[Dict] = []
        self.seg_bs_point_history: List[Dict] = []

    def __deepcopy__(self, memo):
        new_obj = CKLine_List(self.kl_type, self.config)
        memo[id(self)] = new_obj
        for klc in self.lst:
            klus_new = []
            for klu in klc.lst:
                new_klu = copy.deepcopy(klu, memo)
                memo[id(klu)] = new_klu
                if klu.pre is not None:
                    new_klu.set_pre_klu(memo[id(klu.pre)])
                klus_new.append(new_klu)

            new_klc = CKLine(klus_new[0], idx=klc.idx, _dir=klc.dir)
            new_klc.set_fx(klc.fx)
            new_klc.kl_type = klc.kl_type
            for idx, klu in enumerate(klus_new):
                klu.set_klc(new_klc)
                if idx != 0:
                    new_klc.add(klu)
            memo[id(klc)] = new_klc
            if new_obj.lst:
                new_obj.lst[-1].set_next(new_klc)
                new_klc.set_pre(new_obj.lst[-1])
            new_obj.lst.append(new_klc)
        new_obj.bi_list = copy.deepcopy(self.bi_list, memo)
        new_obj.seg_list = copy.deepcopy(self.seg_list, memo)
        new_obj.segseg_list = copy.deepcopy(self.segseg_list, memo)
        new_obj.zs_list = copy.deepcopy(self.zs_list, memo)
        new_obj.segzs_list = copy.deepcopy(self.segzs_list, memo)
        new_obj.bs_point_lst = copy.deepcopy(self.bs_point_lst, memo)
        new_obj.metric_model_lst = copy.deepcopy(self.metric_model_lst, memo)
        new_obj.step_calculation = copy.deepcopy(self.step_calculation, memo)
        new_obj.seg_bs_point_lst = copy.deepcopy(self.seg_bs_point_lst, memo)
        new_obj.bs_point_history = copy.deepcopy(self.bs_point_history, memo)
        new_obj.seg_bs_point_history = copy.deepcopy(self.seg_bs_point_history, memo)
        return new_obj

    @overload
    def __getitem__(self, index: int) -> CKLine: ...

    @overload
    def __getitem__(self, index: slice) -> List[CKLine]: ...

    def __getitem__(self, index: Union[slice, int]) -> Union[List[CKLine], CKLine]:
        return self.lst[index]

    def __len__(self):
        return len(self.lst)

    def cal_seg_and_zs(self):
        if not self.step_calculation:
            self.bi_list.try_add_virtual_bi(self.lst[-1])
        cal_seg(self.bi_list, self.seg_list)
        self.zs_list.cal_bi_zs(self.bi_list, self.seg_list)
        update_zs_in_seg(self.bi_list, self.seg_list, self.zs_list)  # 计算seg的zs_lst，以及中枢的bi_in, bi_out

        cal_seg(self.seg_list, self.segseg_list)
        self.segzs_list.cal_bi_zs(self.seg_list, self.segseg_list)
        update_zs_in_seg(self.seg_list, self.segseg_list, self.segzs_list)  # 计算segseg的zs_lst，以及中枢的bi_in, bi_out

        # 计算买卖点
        self.seg_bs_point_lst.cal(self.seg_list, self.segseg_list)
        self.bs_point_lst.cal(self.bi_list, self.seg_list)
        self._record_current_bs_points()

    def need_cal_step_by_step(self):
        return self.config.trigger_step

    def add_single_klu(self, klu: CKLine_Unit):
        klu.set_metric(self.metric_model_lst)
        if len(self.lst) == 0:
            self.lst.append(CKLine(klu, idx=0))
        else:
            _dir = self.lst[-1].try_add(klu)
            if _dir != KLINE_DIR.COMBINE:  # 不需要合并K线
                self.lst.append(CKLine(klu, idx=len(self.lst), _dir=_dir))
                if len(self.lst) >= 3:
                    self.lst[-2].update_fx(self.lst[-3], self.lst[-1])
                if self.bi_list.update_bi(self.lst[-2], self.lst[-1], self.step_calculation) and self.step_calculation:
                    self.cal_seg_and_zs()
            elif self.step_calculation and self.bi_list.try_add_virtual_bi(self.lst[-1], need_del_end=True):  # 这里的必要性参见issue#175
                self.cal_seg_and_zs()

    def klu_iter(self, klc_begin_idx=0):
        for klc in self.lst[klc_begin_idx:]:
            yield from klc.lst

    def to_dataframes(self) -> Dict[str, pd.DataFrame]:
            dataframes = {}
    
            # Convert lst to DataFrame
            dataframes['kline_list'] = pd.DataFrame([
                {
                    'begin_time': kl.time_begin,
                    'end_time': kl.time_end,
                    'idx': kl.idx,
                    'dir': kl.dir,
                    'high': kl.high,
                    'low': kl.low,
                    'fx': kl.fx
                } for kl in self.lst
            ])
    
            # Convert bi_list to DataFrame
            dataframes['bi_list'] = pd.DataFrame([
                {
                    'begin_time': bi.get_begin_klu().time,
                    'end_time': bi.get_end_klu().time,
                    'idx': bi.idx,
                    'dir': bi.dir,
                    'high':bi._high(),
                    'low':bi._low(),                    
                    'type': bi.type,
                    'is_sure': bi.is_sure,
                    'seg_idx':bi.seg_idx,
                    'parent_seg':bi.parent_seg.idx if bi.parent_seg else None,
                    'begin_klc':bi.begin_klc.idx,
                    'end_klc':bi.end_klc.idx,
                    'begin_val':bi.get_begin_val(),
                    'end_val':bi.get_end_val(),
                    'klu_cnt':bi.get_klu_cnt(),
                    'klc_cnt':bi.get_klc_cnt(),
                } for bi in self.bi_list
            ])
    
            # Convert seg_list to DataFrame
            dataframes['seg_list'] = pd.DataFrame([
                {
                    'begin_time':seg.get_begin_klu().time,
                    'end_time':seg.get_end_klu().time,
                    'idx': seg.idx,
                    'dir': seg.dir,
                    'high': seg._high(),
                    'low': seg._low(),
                    'is_sure': seg.is_sure,
                    'start_bi_idx': seg.start_bi.idx if seg.start_bi else None,
                    'end_bi_idx': seg.end_bi.idx if seg.end_bi else None,
                    'zs_count': len(seg.zs_lst),
                    'bi_count': len(seg.bi_list),
                    'resone':seg.reason,
                } for seg in self.seg_list
            ])
    
            # Convert segseg_list to DataFrame
            dataframes['segseg_list'] = pd.DataFrame([
                {
                    'begin_time':segseg.get_begin_klu().time,
                    'end_time':segseg.get_end_klu().time,                    
                    'idx': segseg.idx,
                    'dir': segseg.dir,
                    'high': segseg._high(),
                    'low': segseg._low(),
                    'is_sure': segseg.is_sure,
                    'start_seg_idx': segseg.start_bi.idx if segseg.start_bi else None,
                    'end_seg_idx': segseg.end_bi.idx if segseg.end_bi else None,
                    'zs_count': len(segseg.zs_lst),
                    'bi_count': len(segseg.bi_list),
                    'resone':segseg.reason,
                } for segseg in self.segseg_list
            ])
    
            # Convert zs_list to DataFrame
            dataframes['zs_list'] = pd.DataFrame([
                {
                    #'idx': zs.idx,
                    #'zs_type': zs.zs_type,
                    'begin_time': zs.begin_bi.get_begin_klu().time,
                    'end_time': zs.end_bi.get_end_klu().time,
                    'high': zs.high,
                    'low': zs.low,
                    'peak_high':zs.peak_high,
                    'peak_low':zs.peak_low,
                    'is_sure': zs.is_sure,
                    'begin_bi_idx': zs.begin_bi.idx if zs.begin_bi else None,
                    'end_bi_idx': zs.end_bi.idx if zs.end_bi else None,
                    'bi_in':zs.bi_in.idx if zs.bi_in else None,
                    'bi_out':zs.bi_out.idx if zs.bi_out else None,
                    'begin_bi_time': zs.begin_bi.get_begin_klu().time if zs.begin_bi else None,
                    'end_bi_time': zs.end_bi.get_begin_klu().time if zs.end_bi else None,
                    'bi_in_time':zs.bi_in.get_begin_klu().time if zs.bi_in else None,
                    'bi_out_time':zs.bi_out.get_begin_klu().time if zs.bi_out else None,
                } for zs in self.zs_list
            ])
    
            # Convert segzs_list to DataFrame
            dataframes['segzs_list'] = pd.DataFrame([
                {
                    #'idx': segzs.idx,
                    #'zs_type': segzs.zs_type,
                    #'begin_time': segzs.begin_time,
                    #'end_time': segzs.end_time,
                    'begin_time': segzs.begin_bi.get_begin_klu().time,
                    'end_time': segzs.end_bi.get_end_klu().time,                    
                    'high': segzs.high,
                    'low': segzs.low,
                    'peak_high':segzs.peak_high,
                    'peak_low':segzs.peak_low,
                    'is_sure': segzs.is_sure,                    
                    'begin_seg_idx': segzs.begin_bi.idx if segzs.begin_bi else None,
                    'end_seg_idx': segzs.end_bi.idx if segzs.end_bi else None,
                    'bi_in':segzs.bi_in.idx if segzs.bi_in else None,
                    'bi_out':segzs.bi_out.idx if segzs.bi_out else None,
                    'begin_bi_time': segzs.begin_bi.get_begin_klu().time if segzs.begin_bi else None,
                    'end_bi_time': segzs.end_bi.get_begin_klu().time if segzs.end_bi else None,
                    'bi_in_time':segzs.bi_in.get_begin_klu().time if segzs.bi_in else None,
                    'bi_out_time':segzs.bi_out.get_begin_klu().time if segzs.bi_out else None,
                } for segzs in self.segzs_list
            ])
    
            # Convert bs_point_lst to DataFrame
            dataframes['bs_point_lst'] = pd.DataFrame([
                {
                    'begin_time': bsp.klu.time,
                    #'idx': bsp.idx,
                    'bsp_type': bsp.type2str(),
                    'bi_idx': bsp.bi.idx if bsp.bi else None,
                    'bi_begin_time': bsp.bi.get_begin_klu().time if bsp.bi else None,
                    'bi_end_time': bsp.bi.get_end_klu().time if bsp.bi else None,
                } for bsp in self.bs_point_lst
            ])
    
            # Convert seg_bs_point_lst to DataFrame
            dataframes['seg_bs_point_lst'] = pd.DataFrame([
                {
                    'begin_time': seg_bsp.klu.time,                    
                    #'idx': seg_bsp.idx,
                    'bsp_type': seg_bsp.type2str(),
                    'seg_idx': seg_bsp.bi.idx if seg_bsp.bi else None,
                    'bi_begin_time': seg_bsp.bi.get_begin_klu().time if seg_bsp.bi else None,
                    'bi_end_time': seg_bsp.bi.get_end_klu().time if seg_bsp.bi else None,
                } for seg_bsp in self.seg_bs_point_lst
            ])
    
            # Add historical bs_points
            dataframes['bs_point_history'] = pd.DataFrame(self.bs_point_history)

            # Add historical seg_bs_points
            dataframes['seg_bs_point_history'] = pd.DataFrame(self.seg_bs_point_history)

            return dataframes
    
    def to_csv(self, directory: str = "output") -> None:
        """
        将所有的 DataFrame 保存为 CSV 文件。

        :param directory: 保存 CSV 文件的目录，默认为 "output"
        """
        # 确保输出目录存在
        os.makedirs(directory, exist_ok=True)

        # 获取所有的 DataFrame
        dataframes = self.to_dataframes()

        # 遍历并保存每个 DataFrame
        for name, df in dataframes.items():
            file_path = os.path.join(directory, f"{name}.csv")
            df.to_csv(file_path, index=False)
            print(f"Saved {name} to {file_path}")

    def _record_current_bs_points(self):
        # Record only the latest bs_points
        if self.bs_point_lst:
            latest_bsp = self.bs_point_lst[-1]
            self.bs_point_history.append({
                'begin_time': latest_bsp.klu.time,
                'bsp_type': latest_bsp.type2str(),
                'is_buy': latest_bsp.is_buy,
                'relate_bsp1': latest_bsp.relate_bsp1.klu.time if latest_bsp.relate_bsp1 else None,
                'bi_idx': latest_bsp.bi.idx if latest_bsp.bi else None,
                'bi_begin_time': latest_bsp.bi.get_begin_klu().time if latest_bsp.bi else None,
                'bi_end_time': latest_bsp.bi.get_end_klu().time if latest_bsp.bi else None,
            })

        # Record only the latest seg_bs_points
        if self.seg_bs_point_lst:
            latest_seg_bsp = self.seg_bs_point_lst[-1]
            self.seg_bs_point_history.append({
                'begin_time': latest_seg_bsp.klu.time,
                'bsp_type': latest_seg_bsp.type2str(),
                'is_buy': latest_seg_bsp.is_buy,
                'relate_bsp1': latest_seg_bsp.relate_bsp1.klu.time if latest_seg_bsp.relate_bsp1 else None,
                'seg_idx': latest_seg_bsp.bi.idx if latest_seg_bsp.bi else None,
                'bi_begin_time': latest_seg_bsp.bi.get_begin_klu().time if latest_seg_bsp.bi else None,
                'bi_end_time': latest_seg_bsp.bi.get_end_klu().time if latest_seg_bsp.bi else None,
            })


def cal_seg(bi_list, seg_list: CSegListComm):
    seg_list.update(bi_list)

    sure_seg_cnt = 0
    if len(seg_list) == 0:
        for bi in bi_list:
            bi.set_seg_idx(0)
        return
    begin_seg: CSeg = seg_list[-1]
    for seg in seg_list[::-1]:
        if seg.is_sure:
            sure_seg_cnt += 1
        else:
            sure_seg_cnt = 0
        begin_seg = seg
        if sure_seg_cnt > 2:
            break

    cur_seg: CSeg = seg_list[-1]
    for bi in bi_list[::-1]:
        if bi.seg_idx is not None and bi.idx < begin_seg.start_bi.idx:
            break
        if bi.idx > cur_seg.end_bi.idx:
            bi.set_seg_idx(cur_seg.idx+1)
            continue
        if bi.idx < cur_seg.start_bi.idx:
            assert cur_seg.pre
            cur_seg = cur_seg.pre
        bi.set_seg_idx(cur_seg.idx)


def update_zs_in_seg(bi_list, seg_list, zs_list):
    sure_seg_cnt = 0
    for seg in seg_list[::-1]:
        if seg.ele_inside_is_sure:
            break
        if seg.is_sure:
            sure_seg_cnt += 1
        seg.clear_zs_lst()
        for zs in zs_list[::-1]:
            if zs.end.idx < seg.start_bi.get_begin_klu().idx:
                break
            if zs.is_inside(seg):
                seg.add_zs(zs)
            assert zs.begin_bi.idx > 0
            zs.set_bi_in(bi_list[zs.begin_bi.idx-1])
            if zs.end_bi.idx+1 < len(bi_list):
                zs.set_bi_out(bi_list[zs.end_bi.idx+1])
            zs.set_bi_lst(list(bi_list[zs.begin_bi.idx:zs.end_bi.idx+1]))

        if sure_seg_cnt > 2:
            if not seg.ele_inside_is_sure:
                seg.ele_inside_is_sure = True


