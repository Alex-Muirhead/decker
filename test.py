#!/usr/bin/env python3
# Copyright 2020 Joel Fenwick
# This script under MIT license (as at https://opensource.org/licenses/MIT)
#  Compares outputs with expected outputs.
import subprocess, sys, os

# Need to parameterise this properly
# Need new directory structure

if len(sys.argv)<3:
    print("Usage: test.py workingdir cmd1 cmd2 ...", file=sys.stderr)
    exit(1)

testdir='tests'
prog=sys.argv[2:]
wd=sys.argv[1]
linecount=0
f=open(os.path.join(testdir, 'testlist'), 'r')
for l in f.readlines():
    linecount+=1
    l=l.strip()
    if l.startswith('#') or len(l)==0:
        continue
    fields=l.split('|')
    if len(fields)<2:
        print("Badly formatted line ", linecount, "\n")
        continue
    proc=subprocess.Popen(prog+fields[2:], executable=prog[0], cwd=wd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out=proc.stdout.readlines()
    out=[str(x.strip(), 'utf-8') for x in out]
    err=proc.stderr.readlines()
    err=[str(x.strip(), 'utf-8') for x in err]
    if len(fields[0])==0:
       expectout=[]     # Need to look at why expectout and expecterr are different
    else:
       expectout=open(os.path.join(testdir, fields[0]),'r').readlines()
       expectout=[x.strip() for x in expectout]
    if len(fields[1])==0:
       expecterr=['']
    else:
       expecterr=open(os.path.join(testdir, fields[1]),'r').readlines()
       expecterr=[x.strip() for x in expecterr]
    if out!=expectout:
       print("stdout mismatch (test defined on line ", linecount, ')\n')
       print("We got:")
       print(out)
       print("We expected:")
       print(expectout)
       continue
    if err!=expecterr:
       print("stderr mismatch (test defined on line ", linecount, ')\n')
       print("We got:")
       print(err)
       print("We expected:")
       print(expecterr)
       continue
