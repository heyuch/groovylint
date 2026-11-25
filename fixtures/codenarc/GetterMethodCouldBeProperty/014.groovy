
            class MyClass {
                @Override
                String getSomething() {         // ignored
                    'something'
                }

                String getSomethingElse() {     // violation
                    'something else'
                }
            }
        