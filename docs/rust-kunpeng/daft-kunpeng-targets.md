## 模板 1

【需求描述】
1、优化Daft的UDF功能，从而提升视频、文本、图像、音频产线的性能
【业务背景与价值】
现有的产线流程中，使用Python定义处理步骤，通过Daft的UDF调用该步骤，进行视频、文本、图像、音频的处理；
优化Daft的UDF能力，有助于提升产线的整体性能
【Actor】
生态开发者
【验收场景】
环境配套
os版本： openeuler 24.03 SP3
daft版本：0.7.5
文本数据集：Common Crawl/RedPajama-Data
图像数据集：LAION-5B/LAION-Art
视频数据集：InternVid
音频数据集：WenetSpeech
【验收方式】
1、基于原始版本的Daft运行视频产线项目，获得原始版Daft产线的吞吐量及响应时间
2、基于优化后的Daft运行视频产线项目，获得优化后Daft产线的吞吐量及相应实现
【验收标准】
验收质量属性：技术项目
1、自提升：950优化前后对比，性能提升5%
2、950 vs 9755：950V200与9755性能比提升5%
【交付计划】
交付件：穿刺源码交付（代码合入GitCode/蓝区代码仓）

## 任务1

修改描述功能，总结模板2 的所有任务， 更新【业务背景与价值】


## 模板2 

【标题】
减少UDF过程中的跨界损耗
【背景】
使用Daft的UDF功能时，执行引擎内部存在大量跨边界的数据移动和格式转换：
1. Rust ↔ Python 边界转换：Rust中的数据类型需要转化为Python UDF能处理的对象，Python UDF的返回值需要变为Daft的内部列数据
2. UDF阶段之间的数据传递
【描述】
通过 减少跨界次数、减少中间物化，减少拷贝，从而降低UDF过程中数据传输开销，从而提升产线性能
【输入】
950V200、9755标准性能验收环境和视频、文本、图像、音频产线性能用例
【输出】
950V200与9755性能对比结果

# 任务2-1

文本归一化优化	融合 (regexp_replace + lower + regexp_replace) 三个本步骤，减少中间 parrequest 结构生成	pipeline B 文本归一化优化-Daft-AtomGit	LLM预处理（相对 3.3% ,  自提升 28%）
MinHash 替换 SVE2 实现	向量化更新多个签名位置，并批处理最多 8 个哈希值，减少循环和内存访问	pipeline B性能优化-文本归一化融合、SVE2 文本向量化、SVE2 MinHash-Daft-AtomGit	LLM预处理（相对提升5% 预估，自提升35%）
LLVM 优化 path 参数优化	优化 llmv 层面自动向量化，针对 950 指定 sve 宽度，充分利用 sve 计算模块能力	target Kunpeng 950 with SVE256-Daft-AtomGit	LLM预处理（相对 3.2%）

# 任务2-2

优化 基准产线  volc_operator_sim 中 一下产线， 目标优化 5%
文本	 网页语料清洗入湖	pipeline_text_fineweb_full_min
视频	长视频场景切分（基模QA产线）	video_scene_split_etl
图像	图像策管全链路	pipeline_image_full_min
音频	ASR 音频准备	audio_asr_prep_canonical
PDF	PDF 文档解析向量化	pipeline_pdf_full_min
自动驾驶	自动驾驶多传感器索引	pipeline_ad_nuscenes_min



# 任务2-3

Daft 适配 鲲鹏亲和多模态算子库	鲲鹏多模态算子库以独立仓库模式交付，Daft 侧集成

# 任务2-4

Daft 适配 Lance 算子下推	Lance 实现数据源侧视频解码和抽帧能力，Daft 侧适配，将算子下推到 Daft 中
