#!/bin/bash

# --- 循环求和 ---
start_time=$(($(date +%s%N)/1000000))
sum=0
for ((i=1; i< 1000000; i++))
do
    sum=$((sum + i))
done
end_time=$(($(date +%s%N)/1000000))
elapsed_time=$((end_time - start_time))
echo "从 1 到 10000 的总和是: $sum"
echo "所需时间: $elapsed_time 毫秒"

# 从 1 到 10000 的总和是: 499999500000
# 所需时间: 2224 毫秒


# --- 循环+条件判断 ---
start_time=$(($(date +%s%N)/1000000))
# 初始化总和
sum=0
# 循环从 1 到 10000
for ((i=1; i<=1000000; i++))
do
    # 判断是否为偶数
    if (( i % 2 == 0 )); then
        sum=$((sum + i))  # 叠加偶数
    fi
done
# 输出结果
end_time=$(($(date +%s%N)/1000000))
elapsed_time=$((end_time - start_time))
echo "从 1 到 10000 的偶数之和是: $sum"
echo "所需时间: $elapsed_time 毫秒"

# 从 1 到 10000 的偶数之和是: 250000500000
# 所需时间: 2274 毫秒


# --- 内存使用情况 ---
initial_memory=$(grep VmRSS /proc/self/status | awk '{print $2}')
sum=0
time {
    for ((i=1; i<=1000000; i++)); do
        sum=$((sum + i))
    done
}
final_memory=$(grep VmRSS /proc/self/status | awk '{print $2}')
echo "从 1 到 1000000 的总和是: $sum"
echo "初始内存使用: ${initial_memory} kB"
echo "结束时内存使用: ${final_memory} kB"
echo "内存使用变化: $((final_memory - initial_memory)) kB"
# real	0m2.205s
# user	0m2.205s
# sys	0m0.000s
# 从 1 到 1000000 的总和是: 500000500000
# 初始内存使用: 6136 kB
# 结束时内存使用: 6112 kB
# 内存使用变化: -24 kB

# --- cpu性能 ---
echo "测试 CPU 性能..."
time {
    for ((i=1; i<=1000000; i++)); do
        ((i * i))
    done
}
# real	0m1.541s
# user	0m1.541s
# sys	0m0.000s
