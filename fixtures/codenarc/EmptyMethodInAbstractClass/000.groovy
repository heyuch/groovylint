
            abstract class MyClass {
                abstract def method1()

                def method2() {
                    return "value"
                }

                private void method3() {
                    // private is OK
                }
                private void method4() {
                    return null // private is OK
                }

                void method5() {
                    println '...'   // has implementation
                }
                def method6() {
                    "value" // implicit return
                }
            }

            class MyOtherClass {
                void method1() {
                    // OK because not in abstract class
                }
                private void method4() {
                    return null // OK because not in abstract class
                }
            }
        