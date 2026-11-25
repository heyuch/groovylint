
            class MyClass {
                private String myMethod1(String id, int value) { doSomething(value); return id }
                private void myMethod2(int value) { def x = value }
                private def myMethod3(Date startDate) { return "${startDate}" }
                private def myMethod4(Date startDate) {
                    return new Object() {
                        def x = startDate
                    }
                }
                private def myMethod5(Date startDate) {
                    return new Object() {
                        String toString() { return startDate }
                    }
                }
            }
        