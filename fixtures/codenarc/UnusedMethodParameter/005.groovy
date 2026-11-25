
            class MyClass {
                void myMethod(int value) { }

                // this is NOT a reference, but the AST does not have enough information for this
                def x = new Object() { def y = value  }
            }
        