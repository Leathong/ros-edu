import re
import subprocess
import argparse

def extract_addresses(line):
    # 使用正则表达式提取 ra 后面的地址
    match = re.search(r'(^\s*\d{1,3}:)\s+.+ra:\s+(0x[0-9a-fA-F]+)', line)
    if match:
        return match
    return None

def addr2line(address, executable):
    # 调用 addr2line 工具解析地址
    tool = '/Volumes/ZHITAI/open_source/ANOS/toolchain/riscv64-unknown-elf-toolchain/bin/riscv64-unknown-elf-addr2line'
    result = subprocess.run([tool, '-e', executable, '-a', address], stdout=subprocess.PIPE)
    return result.stdout.decode('utf-8').strip()

def parse_stack_trace(crash_file, executable):
    with open(crash_file, 'r') as file:
        for line in file:
            match = extract_addresses(line)
            if match:
                idx = match.group(1)
                address = match.group(2)
                line_info = addr2line(address, executable)
                print(f"{idx} {line_info.split()[1]}")
            else:
                # print(line)
                pass

def main():
    # 使用 argparse 解析命令行参数
    parser = argparse.ArgumentParser(description="Parse a crash file and resolve addresses using addr2line.")
    parser.add_argument('-c', '--crash_file', type=str, help="Path to the crash file containing the stack trace.")
    parser.add_argument('-e', '--executable', type=str, help="Path to the executable file used to generate the stack trace.")
    args = parser.parse_args()

    # 调用解析函数
    parse_stack_trace(args.crash_file, args.executable)

if __name__ == "__main__":
    main()
