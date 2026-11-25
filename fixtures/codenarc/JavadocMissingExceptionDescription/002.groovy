
        class MyClass {
            /**
             * Return the calculated count of some stuff.
             *
             * @exception name
             *
             * NOTE: Only the first occurrence of an empty @return tag is found.
             *       So the following line is not flagged as a violation!!!
             * @exception name2
             */
            int countThings(int startIndex) { }

            /**
             *@exception otherName
             */
            String otherMethod() { }
        }
        