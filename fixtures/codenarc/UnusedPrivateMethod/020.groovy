
            class MyClass {
                private int fireEvent(int index) { }
                private int fireEvent(String name) { }
                private int fireEvent(int index, String name) { }
                def other = this.fireEvent(object)
            }
        