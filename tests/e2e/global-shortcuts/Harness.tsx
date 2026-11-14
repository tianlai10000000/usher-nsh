/** Starts production global initialization with an isolated window declaration. */
import { mount } from "nsh";
import { Content } from "./Content";
import { fixture } from "./fixture";
if (!fixture.available()) throw new Error("test_global_key_not_released_before_startup");
mount(Content);
