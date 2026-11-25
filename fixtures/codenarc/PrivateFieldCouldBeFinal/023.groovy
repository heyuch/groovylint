
            class MyOtherClass {
                private count

                def doStuff() {
                    count = new MyClass(count)
                }
            }

            class MyClass {
                private int count

                protected MyClass(int count) {
                    this.count = count
                }
            }
            