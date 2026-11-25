
            class MyClass {
                MyClass() { }
                MyClass(String id) { println id }

                String myMethod1(String id, int value) { doSomething(value); return id }
                void myMethod2(int value) { def x = value }
                def myMethod3(Date startDate) { return "${startDate}" }
                def myMethod4(Date startDate) {
                    return new Object() {
                        def x = startDate
                    }
                }
                def myMethod5(Date startDate) {
                    return new Object() {
                        String toString() { return startDate }
                    }
                }
            }
        