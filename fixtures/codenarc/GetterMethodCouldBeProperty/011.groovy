
            class MyClass {
                static value = 'value'

                String getSomethingElse() {
                    value               // We can't be sure this value won't change
                }
            }
        