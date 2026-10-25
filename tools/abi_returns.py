"""Return-channel rules shared by the byte and Ghidra-export ABI extractors.

Standard library only. An EAX write does not establish a C/C++ return type.
"""

X87_PUSH = ("fld", "fild", "fbld", "fldz", "fld1", "fldpi", "fldl2e", "fldl2t", "fldlg2", "fldln2",
            "fptan", "fsincos", "fxtract")
X87_POP1 = ("fstp", "fistp", "fisttp", "fbstp", "fcomp", "fucomp", "fcomip", "fucomip", "ficomp",
            "faddp", "fsubp", "fsubrp", "fmulp", "fdivp", "fdivrp", "fyl2x", "fyl2xp1", "fpatan", "ffreep")
X87_POP2 = ("fcompp", "fucompp")


def ret_label(kinds, pops, notes=()):
    """The vtables.tsv vocabulary; shared with abi_extract.py, including uncertainty."""
    if not pops and any(n.startswith("indirect-jump") for n in notes):
        return "forward", "medium"

    def strip(x):
        while x.startswith("callee:"):
            x = x[len("callee:"):]
        return x

    k = {strip(x) for x in kinds}
    k -= {x for x in k if x.startswith("x87-depth:")}
    if not pops:
        return "noreturn?", "low"
    k.discard("via-call")
    k.discard("unknown")
    if k == {"x87"}:
        return "x87", "high"
    if "x87" in k:
        return "x87|" + "|".join(sorted(k - {"x87"})), "low"
    if k == {"edx:eax"}:
        return "edx:eax", "medium"
    if k == {"eax"}:
        return "eax-or-void", "medium"
    if "edx:eax" in k:
        return "edx:eax?", "low"
    if not k:
        return ("via-call" if "via-call" in kinds else "unknown"), "low"
    return "|".join(sorted(k)), "low"
