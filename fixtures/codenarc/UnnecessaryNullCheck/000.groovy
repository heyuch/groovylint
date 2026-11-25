
            // null check it OK
            if (obj != null) { }

            // null safe dereference in if is OK
            if (obj?.method()) { }

            // null safe dereference in ternary is OK
            (obj?.prop && obj?.prop2) ? x : y

            // obj is reused in a parameter list, so OK
            if (obj != null && obj.method() && isValid(obj)) { }

            // ok if changed
            if (obj != null && obj.method().method2()) { }
            if (obj != null && obj.prop1.prop2) { }

            // ok, different properties
            if (obj.prop1 && obj.prop2 != null) {  }
            if (obj.method1() && obj.method2() != null) {  }

            // ok, this is qualified
            if (this.x == null) {}
        