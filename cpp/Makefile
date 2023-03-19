OPTS=-g -Wall -pedantic -std=c++20 -Werror

CC=g++

decker: Card.o  CardCollection.o  Cost.o  main.o  Pile.o  property.o Constraint.o rand.o Selection.o util.o actions.o Config.o costtarget.o CostVotes.o
	$(CC) -o decker Card.o  CardCollection.o  Cost.o  main.o  Pile.o  property.o Constraint.o rand.o Selection.o util.o actions.o Config.o costtarget.o CostVotes.o

costtarget.o: costtarget.cc costtarget.h types.h rand.h
	$(CC) $(OPTS) -c costtarget.cc

Card.o: Card.cc types.h rand.h
	$(CC) $(OPTS) -c Card.cc

Selection.o: Selection.cc types.h rand.h
	$(CC) $(OPTS) -c Selection.cc

CardCollection.o: CardCollection.cc types.h rand.h property.h util.h actions.h
	$(CC) $(OPTS) -c CardCollection.cc

Cost.o: Cost.cc types.h rand.h
	$(CC) $(OPTS) -c Cost.cc

main.o: main.cc types.h rand.h property.h Config.h
	$(CC) $(OPTS) -c main.cc

Pile.o: Pile.cc types.h rand.h
	$(CC) $(OPTS) -c Pile.cc

property.o: property.cc property.h types.h rand.h
	$(CC) $(OPTS) -c property.cc

Constraint.o: Constraint.cc types.h rand.h property.h
	$(CC) $(OPTS) -c Constraint.cc

rand.o: rand.h rand.cc
	$(CC) $(OPTS) -c rand.cc

util.o: util.cc property.h types.h rand.h util.h
	$(CC) $(OPTS) -c util.cc

actions.o: actions.cc actions.h types.h rand.h property.h
	$(CC) $(OPTS) -c actions.cc

Config.o: Config.cc Config.h rand.h types.h util.h costtarget.h property.h actions.h
	$(CC) $(OPTS) -c Config.cc

CostVotes.o: CostVotes.cc types.h rand.h
	$(CC) $(OPTS) -c CostVotes.cc
clean:
	rm -f *.o decker
