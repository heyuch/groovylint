
            /**Bad1
             *Ignored .. not a violation
             *    @author Some Developer
             *
             */
            class MyClass {//Bad4
                /**
                 *Return the calculated count of some stuff,
                 */
                int countThings(int startIndex) { //some comment
                    //Do stuff
                    amount = 3 + amount/*violation*/
                }//comment
            }
            //Other comment
            /*Single line comment*/
        