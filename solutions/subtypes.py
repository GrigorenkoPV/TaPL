#!/usr/bin/env python

import itertools
from abc import ABC, abstractmethod
from dataclasses import dataclass


class Type(ABC):
    # Excluding Self and Bot
    @abstractmethod
    def subtypes(self): ...

    # Excluding Self and Top
    @abstractmethod
    def supertypes(self): ...
    @abstractmethod
    def __str__(self): ...


@dataclass
class Simple(Type):
    def subtypes(self):
        yield from [self, Bot()]

    def supertypes(self):
        yield from [self, Top()]


class Bot(Simple):
    def subtypes(self):
        yield from [self]

    def __str__(self):
        return "Bot"


class Top(Simple):
    def supertypes(self):
        yield from [self]

    def __str__(self):
        return "Top"


class Nat(Simple):
    def __str__(self):
        return "Nat"


class Bool(Simple):
    def __str__(self):
        return "Bool"


@dataclass
class List[T: Type](Type):
    inner: T

    def __str__(self):
        return f"List[{self.inner}]"

    def subtypes(self):
        yield from map(List, self.inner.subtypes())
        yield Bot()

    def supertypes(self):
        yield from map(List, self.inner.supertypes())
        yield Top()


@dataclass
class Fn[A: Type, R: Type](Type):
    arg: A
    ret: R

    def __str__(self):
        return f"({self.arg} → {self.ret})"

    def subtypes(self):
        for sup_arg in self.arg.supertypes():
            for sub_ret in self.ret.subtypes():
                yield Fn(sup_arg, sub_ret)
        yield Bot()

    def supertypes(self):
        for sub_arg in self.arg.subtypes():
            for sup_ret in self.ret.supertypes():
                yield Fn(sub_arg, sup_ret)
        yield Top()


@dataclass
# not really correct, technically we have to synthesize inetersection of sub and super for inner
# but good enough for this task
class Ref[T: Type](Simple):
    inner: T

    def __str__(self):
        return f"Ref {self.inner}"


def flatmap(f, l):
    if l:
        name, head = l[0]
        tail = l[1:]
        for ftail in flatmap(f, tail):
            for fhead in f(head):
                yield [(name, fhead)] + ftail
    else:
        yield []


@dataclass
class Variant[T: Type](Type):
    options: list[tuple[str, T]]

    def __str__(self):
        return "<" + ", ".join(f"{name} : {ty}" for (name, ty) in self.options) + ">"

    def subtypes(self):
        for r in range(len(self.options) + 1):
            for options in itertools.permutations(self.options, r):
                for suboptions in flatmap(lambda x: x.subtypes(), options):
                    yield Variant(suboptions)
        yield Bot()

    def supertypes(self):
        raise RuntimeError("Variants have an unlimited number of supertypes")


@dataclass
class Record[T: Type](Type):
    fields: list[tuple[str, T]]

    def __str__(self):
        return "\\{" + ", ".join(f"{name} : {ty}" for (name, ty) in self.fields) + "\\}"

    def subtypes(self):
        raise RuntimeError("Records have an unlimited number of subtypes")

    def supertypes(self):
        for r in range(len(self.fields) + 1):
            for fields in itertools.permutations(self.fields, r):
                for supfields in flatmap(lambda x: x.supertypes(), fields):
                    yield Record(supfields)
        yield Top()


def print_all(t: Type):
    for num, sub in enumerate(t.subtypes(), start=1):
        print(f"      {num}. $\\code{{{sub}}}$\n")


def variant(**kwargs):
    return Variant(list(dict(kwargs).items()))


def record(**kwargs):
    return Record(list(dict(kwargs).items()))


if __name__ == "__main__":
    for t in [
        Fn(
            record(
                a=Fn(
                    variant(b=Nat()),
                    Nat(),
                )
            ),
            variant(e=variant(f=Bool())),
        ),
        Fn(
            List(record(a=Nat(), b=Bool())),
            List(variant(a=Nat(), b=Bool())),
        ),
        Fn(
            Fn(
                variant(
                    a=Fn(
                        Ref(record(b=Top())),
                        Bot(),
                    )
                ),
                Top(),
            ),
            Bot(),
        ),
    ]:
        print_all(t)
        print()
        print()
