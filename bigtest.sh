#!/bin/bash

echo "Starting batch 0"
for seed in {77777..77977}
do
    for opt in '' --boxes=Adventures --boxes=Guilds --boxes=Dominion2 --boxes=Empires --boxes=Hinterlands --boxes=Renaissance --boxes=Seaside --boxes=Menagerie --boxes=Guilds,Menagerie
    do
        rm -f /tmp/c /tmp/r
        ../cpp/decker --badrand --boxfile=boxes.dat $opt --seed=$seed > /tmp/c &
        ../rs/decker --badrand --boxfile=boxes.dat $opt --seed=$seed > /tmp/r &
        wait
        diff -q /tmp/c /tmp/r > /dev/null
        if [ $? != 0 ]
        then
            head -1 /tmp/c
        fi
    done

done



echo "Starting batch 1"
for seed in {5555..6555}
do
  for opt in '' --min-type=Treasure:5 --max-type=Treasure:3 --max-cost-repeat=3 --max-cost-repeat=4 --max-prefixes=3 --landscape-count=3 --landscape-count=4 --exclude=Colony
  do
        rm -f /tmp/c /tmp/r
        ../cpp/decker --badrand $opt --seed=$seed > /tmp/c &
        ../rs/decker --badrand $opt --seed=$seed > /tmp/r &
        wait
        diff -q /tmp/c /tmp/r > /dev/null
        if [ $? != 0 ]
        then
            head -1 /tmp/c
        fi
  done
done

echo "Starting batch 2"
for seed in {10001..10051}
do
    for inc in '' '--no-attack-react' '--no-anti-cursor'
    do
        ../cpp/decker --badrand --min-type=Attack:2 --include=Quest --seed=$seed $inc > /tmp/c;
        ../rs/decker --badrand --min-type=Attack:2 --include=Quest --seed=$seed $inc > /tmp/r;
        diff -q /tmp/c /tmp/r > /dev/null
        if [ $? != 0 ]
        then
            head -1 /tmp/c
        fi
    done
done

echo "Starting batch 3"
for seed in {1..1218}
do
for inc in '' --include=Page --include=Rats,Tournament;
do
   ../cpp/decker --badrand --seed=$seed $inc > /tmp/c;
   ../rs/decker --badrand --seed=$seed $inc > /tmp/r;
   diff /tmp/c /tmp/r
   if [ $? != 0 ]
   then
      head -1 /tmp/c
   fi
done
done
