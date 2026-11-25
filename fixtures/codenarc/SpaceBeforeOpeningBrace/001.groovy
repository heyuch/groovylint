
            class MyClass /* emoji 💩 */ {
                def myMethod() /* emoji 💩 */ {
                    if (someString.contains("emoji 💩 ")) { }
                    while(someString.contains("emoji 💩 ")) { }
                    poo("with an emoji 💩 should not break") { }
                    poo("with unicode 𝓈 should not break") { }
                }
            }
        