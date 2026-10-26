// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4DrawStream.h"
#include "FC4DrawStreamCapture.h"
#include <cassert>
#include <thread>
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

    Stream captured;
    int32_t rect[] = {1, 2, 30, 40};
    {
        Capture scope(&context, captured, objects);
        assert(Capture::Current(&context) == &scope && !Capture::Current(&image));
        std::thread other([&] { assert(!Capture::Current(&context)); });
        other.join();
        scope.Before(35, {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(rect))});
        rect[0] = 99;
        scope.After(13, {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(rect))}, 0);
        {
            Capture nested(&image, captured, objects);
            assert(Capture::Current(&image) == &nested && !Capture::Current(&context));
        }
        assert(Capture::Current(&context) == &scope);
    }
    assert(!Capture::Current(&context) && !captured.Valid()); // partial capture has no credit
    assert(captured.Events().size() == 2 && captured.Events()[0].words.empty());
    int32_t copied;
    std::memcpy(&copied, captured.Events()[0].bytes.data(), sizeof(copied));
    assert(copied == 1); // input payload owned by trace, not a borrowed rectangle
    std::memcpy(&copied, captured.Events()[1].bytes.data(), sizeof(copied));
    assert(copied == 99); // out-parameter observed after native execution
    Stream tooManyPoints;
    {
        Capture scope(&context, tooManyPoints, objects);
        scope.Before(44, {8, 1, 0x20000001}); // must not wrap to an eight-byte read on x86
    }
    assert(!tooManyPoints.Valid() && tooManyPoints.Events().size() == 1 && tooManyPoints.Events()[0].bytes.empty());

    // A core pass computes its own draw arguments and obtains only the native
    // getter responses, without executing any borrowed object's method twice.
    Stream observed, replayed;
    Inputs getterInputs;
    const auto word = [](const void* pointer) { return static_cast<uint32_t>(reinterpret_cast<uintptr_t>(pointer)); };
    int32_t nativeClip[] = {10, 20, 100, 200}, coreClip[4]{};
    {
        Capture scope(&context, observed, objects, &getterInputs);
        scope.Before(13, {word(nativeClip)});
        scope.After(13, {word(nativeClip)}, 0xdeadbeef); // void: ignore scratch EAX
        scope.Before(18, {});
        scope.After(18, {}, word(&image)); // borrowed engine, no AddRef
        scope.Before(21, {0xff123456});
        scope.Before(35, {word(nativeClip)});
        scope.Before(38, {word(&image), word(nativeClip), word(nativeClip)});
    }
    nativeClip[0] = -999; // replay must use the captured value, not this memory
    ContextReplay dcReplay(&context, replayed, objects, getterInputs);
    dcReplay.Invoke(13, {word(coreClip)});
    assert(coreClip[0] == 10 && coreClip[3] == 200);
    assert(dcReplay.Invoke(18) == word(&image));
    dcReplay.Invoke(21, {0xff123456});
    dcReplay.Invoke(35, {word(coreClip)});
    dcReplay.Invoke(38, {word(&image), word(coreClip), word(coreClip)});
    assert(getterInputs.Complete() && replayed.Valid());
    assert(observed.Events() == replayed.Events());
    assert(!Compare(observed, replayed, All, getterInputs).complete); // no owner/font/surface credit
    dcReplay.Invoke(13, {word(coreClip)}); // extra getter cannot call the game
    assert(!replayed.Valid() && !getterInputs.Complete());

    Stream unsupported, malformed;
    Inputs noInputs;
    ContextReplay unsupportedReplay(&context, unsupported, objects, noInputs);
    unsupportedReplay.Invoke(27, {1, 2, 3}); // no settled gradient transport
    assert(!unsupported.Valid());
    ContextReplay malformedReplay(&context, malformed, objects, noInputs);
    malformedReplay.Invoke(35); // missing rectangle must not dereference words[0]
    assert(!malformed.Valid());

    Inputs bounded(1, 8);
    bounded.Observe(request, {Domain::Return, 5, 7, {50}, {}});
    bounded.Observe(request, {Domain::Return, 5, 7, {50}, {}});
    assert(!bounded.Complete());
    Inputs boundedBytes(1, 7);
    boundedBytes.Observe(request, {Domain::Return, 5, 7, {50}, {}});
    assert(!boundedBytes.Complete() && !boundedBytes.Read(request));
    Stream a, b;
    a.Call(Domain::Surface, 6, 1, {0x8040, 1});
    b.Call(Domain::Surface, 6, 1, {0x8040, 1});
    a.Call(Domain::Surface, 100, 1, {10, 20});
    b.Call(Domain::Surface, 100, 1, {10, 21});
    const auto difference = Inspect(a, b);
    assert(difference.operation == 1 && difference.element == 1);
    assert(difference.native == 20 && difference.core == 21 && difference.exactOperations == 1);
    assert(std::strcmp(difference.field, "word") == 0);
    Stream empty;
    assert(std::strcmp(Inspect(empty, empty).field, "none") == 0);
    assert(std::strcmp(Inspect(a, empty).field, "operation-count") == 0);
    return 0;
}
