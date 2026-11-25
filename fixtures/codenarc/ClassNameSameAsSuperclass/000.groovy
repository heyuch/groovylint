
            class MyClass { }
            class Integer { }
            class MyOtherClass extends SomeOtherClass { }

            def innerClass = new MyClass() {
                int getCount() { return 99 }
            }
        