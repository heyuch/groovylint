
            class MyClass {
                private int myMethod() { }
                private int otherMethod() { }
                def getValue() { return this.&myMethod }
            }
        