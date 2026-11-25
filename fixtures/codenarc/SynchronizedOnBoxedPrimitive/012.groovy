
                class MyClass {

                    final Boolean boollock = true

                    class MyInnerClass {
                        def method() {
                            // violation
                            synchronized(boollock) { }
                        }
                    }
                }
            