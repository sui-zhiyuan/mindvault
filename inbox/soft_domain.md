
cat /sys/kernel/debug/sched/features
enable 

cat /sys/fs/cgroup/cpu/<cgroup>/cpu.soft_domain_nr_cpu 
修改为 1

cat /sys/fs/cgroup/cpu/<cgroup>/cpu.soft_domain
修改为 -1


cgroup 位置

/sys/fs/cgroup/cpu,cpuacct/cube_sandbox/sandbox


uv run softdomain_ctl.py enable --mode pushdown --level both --apply

uv run softdomain_ctl.py disable --mode pushdown --level both --apply