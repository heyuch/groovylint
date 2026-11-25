
                class MyClass {
                    def myMethod() {
                        [1,2]as String
                        { -> println 123 } as Runnable      // ok
                        { -> println 456 }as
                            Runnable
                        { -> println 789
                             }as Runnable
                        (int)34.56                          // ignored
                    }
                }
            