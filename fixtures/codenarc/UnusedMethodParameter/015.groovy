
            class MyClass1 {
                void main(String[] args) { }    // not static
            }
            class MyClass2 {
                static main(arg1, arg2) { }     // too many args
            }
            class MyClass3 {
                static main(int value) { }     // wrong arg type
            }
            class MyClass4 {
                static int main(String[] args) { }     // wrong return type
            }
        