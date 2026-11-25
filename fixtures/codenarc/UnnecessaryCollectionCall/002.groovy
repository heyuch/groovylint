
            class MyClass {
                def x = [1, 2, 3]

                def foo() {
                    x.containsAll(x)
                    x.retainAll(x)
                }
            }
        