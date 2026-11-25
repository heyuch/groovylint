
            def foo() {}
            def foo(int x) {}
            class MyClass {
                def foo() {}
                def foo(int x) {}
                def x = new Object() {
                    def foo() {}
                    def foo(int x) {}
                    def y = new Object() {
                        def foo = {}
                        def FoO = {}
                        def foO() {}
                    }
                }
            }
        