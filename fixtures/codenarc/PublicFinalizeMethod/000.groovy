
            class MyClass1 {
                protected finalize() {}

                public finalize(String arg) {
                    // overloading finalize is a bad practice, but OK for this rule
                }
            }
        