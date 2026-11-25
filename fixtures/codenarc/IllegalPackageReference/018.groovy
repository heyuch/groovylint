
            import com.example.ExampleHelper
            class MyClass implements com.example.print.Printable {
                def myMethod() { println 'ok' }
                int getCount(org.bad.widget.Widget widget) { return widget.count }
            }
        