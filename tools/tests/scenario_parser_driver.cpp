// Test the generator against the plugin's actual header-only parser.
#include "FC4HarnessLogic.h"
#include <iostream>
#include <iterator>
#include <string>

int main() {
    std::string script;
    size_t scripts = 0, actions = 0, errors = 0;
    while (std::getline(std::cin, script, '\0')) {
        const auto parsed = fc4::harness::ParseScript(script);
        ++scripts;
        actions += parsed.actions.size();
        for (const auto& error : parsed.errors) {
            ++errors;
            if (errors <= 10)
                std::cerr << "script " << scripts << " line " << error.line
                          << ": " << error.message << '\n';
        }
    }
    std::cout << "scripts=" << scripts << " actions=" << actions
              << " parse_errors=" << errors << '\n';
    return errors ? 1 : 0;
}
