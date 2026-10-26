// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4DrawStream.h"
#include <cassert>
using namespace fc4::drawstream;

int main() {
    Objects objects;
    int context = 0, image = 0;
    const auto dc = objects.Borrow(&context), img = objects.Borrow(&image);
    assert(dc != img && objects.Borrow(&context) == dc && objects.Borrow(nullptr) == 0);
    objects.Image(&image, {1, 2, 3});
    Stream native, core;
    for (auto* stream : {&native, &core}) {
        stream->Call(Domain::Context, 8, dc, {0, 0, 100, 200});
        stream->Call(Domain::Context, 21, dc, {0xff123456});
        stream->Image(Domain::Context, 38, dc, objects, img, {0, 0, 10, 20});
        const char text[] = "same\0draw";
        stream->Data(Domain::Font, 1, 7, text, sizeof(text), {10, 20, 0xffabcdef});
        stream->Call(Domain::Command, 42, 9, {100}); // record only, never execute
        stream->Call(Domain::State, 0x100, dc, {3});
        stream->Call(Domain::Return, 0, dc, {1});
    }
    Inputs inputs;
    const Event request{Domain::Query, 5, 7, {10}, {}};
    inputs.Observe(request, {Domain::Return, 5, 7, {50}, {}});
    assert(!Compare(native, core, All, inputs).complete);
    assert(inputs.Read(request)->words[0] == 50);
    assert(Compare(native, core, All, inputs).exact);
    assert(!Compare(native, core, Return | Collaborators, inputs).complete);
    core.Call(Domain::Message, 1, 2, {3});
    const auto different = Compare(native, core, All, inputs);
    assert(different.complete && !different.exact && different.firstDifference == 7);
    Method method;
    method.Observe(different);
    method.Observe(Compare(native, core, Return, inputs));
    assert(method.calls == 2 && method.complete == 1 && method.mismatches == 1 && method.incomplete == 1);
    Stream unkeyed;
    unkeyed.Image(Domain::Surface, 29, dc, objects, dc, {});
    assert(!unkeyed.Valid());
    Stream small(1, 8);
    small.Call(Domain::Context, 1, dc, {1, 2});
    small.Call(Domain::Context, 2, dc);
    assert(!small.Valid() && small.Events().size() == 1);
    Stream bytes(2, 1);
    bytes.Data(Domain::Font, 0, dc, "ab", 2);
    assert(!bytes.Valid() && bytes.Events().empty());
    assert(!inputs.Read(request) && !inputs.Complete());
    assert(FloatWord(-0.0f) != FloatWord(0.0f));
    return 0;
}
