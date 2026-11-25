
            class MyTestCase {
                def myMethod() {
                    println 1;println 2
                    int i=0;i++;println i
                    def closure = { x -> println x;x = 23; }
                }
            }
        