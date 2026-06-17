#!/bin/dash

start_time=$(($(date +%s%N)/1000000))
sum=0
for i in $(seq 1 1000000); do
    sum=$((sum + i))
done
end_time=$(($(date +%s%N)/1000000))
elapsed_time=$((end_time - start_time))
# 输出结果
echo "Sum: $sum"
echo "Elapsed time: $elapsed_time seconds"