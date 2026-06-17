set count 1000
function timed
    # 获取开始时间戳
    set START_TS (date +%s%N)

    # 执行百万次循环求和
    set sum 0
    for i in (seq $count)
        set sum (math $sum + $i)
    end

    # 计算运行时间
    set END_TS (date +%s%N)
    set RUNTIME (math "($END_TS - $START_TS)/1000000")
    echo "sum of 1 to $count is: $sum" 
    echo "$count次循环耗时: $RUNTIME 毫秒"
end

# 执行测试函数
timed
